#!/usr/bin/env bash
# THE SERVER SIDE of the deploy. Never run this from your laptop.
#
# deploy.sh, rollback.sh and seed.sh upload this file before every run and then call
# one of its subcommands over SSH. It exists so that how the app is run (its systemd
# unit and environment) lives in exactly one place: a rollback must start the app the
# same way as a deploy, and `run import-focuses` must see the same database.
#
# Usage: bash remote.sh <init | nginx_reload | activate VERSION | rollback | seed
#                        | prune | status | health | versions>
set -euo pipefail

# Defaults mirror deploy/config.sh; the caller always overrides them via env.
APP_NAME="${APP_NAME:-hoi4_planner}"
REMOTE_DIR="${REMOTE_DIR:-/home/ced/rust/hoi4_planner}"
SERVICE_NAME="${SERVICE_NAME:-hoi4_planner}"
URL_PREFIX="${URL_PREFIX:-/hoi4}"
KEEP_RELEASES="${KEEP_RELEASES:-3}"
SOCKET_DIR="${SOCKET_DIR:-/home/ced/services/nginx_proxy/sockets}"
NGINX_SOCKET_DIR="${NGINX_SOCKET_DIR:-/sockets}"
SOCKET_NAME="${SOCKET_NAME:-hoi4_planner.sock}"
NGINX_CONTAINER="${NGINX_CONTAINER:-nginx-proxy}"
NGINX_CONF_DIR="${NGINX_CONF_DIR:-/home/ced/services/nginx_proxy/sites/apps}"
NGINX_CONF_NAME="${NGINX_CONF_NAME:-hoi4_planner.conf}"

RELEASES_DIR="$REMOTE_DIR/releases"
DB_DIR="$REMOTE_DIR/data/db"
CURRENT_LINK="$REMOTE_DIR/current"
CURRENT_FILE="$REMOTE_DIR/current_version.txt"
PREVIOUS_FILE="$REMOTE_DIR/previous_version.txt"
ENV_FILE="$REMOTE_DIR/app.env"
RUN_SCRIPT="$REMOTE_DIR/run"
UNIT_DIR="$HOME/.config/systemd/user"
UNIT_FILE="$UNIT_DIR/$SERVICE_NAME.service"
SOCKET_PATH="$SOCKET_DIR/$SOCKET_NAME"

die() { echo "error: $*" >&2; exit 1; }

read_version() { [ -f "$1" ] && cat "$1" || true; }

# --- environment -----------------------------------------------------------

# The app's whole environment, in one file read by the unit (EnvironmentFile) AND by
# `run`, so a command always acts on the database the server uses.
#
# ASSET_ROOT follows `current`, which is where a release keeps static/ and data/. The
# database lives outside the releases, as a bare path, and survives every deploy.
write_env() {
  cat > "$ENV_FILE.tmp" <<ENV
DATABASE_URL=$DB_DIR/$APP_NAME.db
ASSET_ROOT=$CURRENT_LINK
SOCKET_PATH=$SOCKET_PATH
URL_PREFIX=$URL_PREFIX
RUST_LOG=info
ENV
  mv "$ENV_FILE.tmp" "$ENV_FILE"

  # `$REMOTE_DIR/run <command>` runs the current binary with that environment:
  #   ssh nuc150 /home/ced/rust/hoi4_planner/run import-focuses --from-json
  cat > "$RUN_SCRIPT" <<RUN
#!/usr/bin/env bash
set -a; . '$ENV_FILE'; set +a
exec '$CURRENT_LINK/$APP_NAME' "\$@"
RUN
  chmod +x "$RUN_SCRIPT"
}

# The systemd user unit. Rewritten every time, so a change here reaches the server
# with the next deploy or rollback; daemon-reload only when it changed.
write_unit() {
  mkdir -p "$UNIT_DIR"
  cat > "$UNIT_FILE.tmp" <<UNIT
[Unit]
Description=HOI4 planner ($SERVICE_NAME)
After=network-online.target

[Service]
WorkingDirectory=$REMOTE_DIR
EnvironmentFile=$ENV_FILE
ExecStart=$CURRENT_LINK/$APP_NAME serve
Restart=on-failure
RestartSec=2

[Install]
WantedBy=default.target
UNIT
  if cmp -s "$UNIT_FILE.tmp" "$UNIT_FILE"; then
    rm -f "$UNIT_FILE.tmp"
  else
    mv "$UNIT_FILE.tmp" "$UNIT_FILE"
    systemctl --user daemon-reload
  fi
  systemctl --user enable "$SERVICE_NAME" >/dev/null 2>&1
}

# --- nginx -----------------------------------------------------------------

# The socket directory belongs to nginx_proxy, which bind-mounts it into its
# container — we only use it. Deliberately NOT created here: if it is missing when
# the proxy's compose starts, docker creates it as root and the app can no longer
# write its socket.
check_socket_dir() {
  [ -d "$SOCKET_DIR" ] || die "$SOCKET_DIR does not exist.$(socket_dir_help)"
  [ -w "$SOCKET_DIR" ] || die "$SOCKET_DIR is not writable by $(id -un)."

  local mounted
  mounted="$(docker inspect "$NGINX_CONTAINER" \
    --format '{{range .Mounts}}{{if eq .Source "'"$SOCKET_DIR"'"}}{{.Destination}}{{end}}{{end}}' \
    2>/dev/null)" || die "no container named '$NGINX_CONTAINER' — is the proxy running?"
  [ "$mounted" = "$NGINX_SOCKET_DIR" ] || die \
    "$NGINX_CONTAINER does not mount $SOCKET_DIR on $NGINX_SOCKET_DIR (currently: '${mounted:-nothing}').$(socket_dir_help)"
}

socket_dir_help() {
  cat <<HELP

Once, in the nginx_proxy project:
  mkdir -p $SOCKET_DIR        (as $(id -un), BEFORE restarting the proxy)
  docker-compose.yml, service nginx, volumes:
    - $SOCKET_DIR:$NGINX_SOCKET_DIR:ro
  docker compose up -d        (recreates the container with the new mount)
HELP
}

# The app may answer on the host while nginx still cannot see its socket (mount
# missing or pointing elsewhere): check from inside the container too.
probe_nginx_view() {
  docker exec "$NGINX_CONTAINER" test -S "$NGINX_SOCKET_DIR/$SOCKET_NAME" 2>/dev/null
}

# --- app -------------------------------------------------------------------

# Point `current` at a release and (re)start the unit on it. This is the single
# source of truth for how the app is run.
start_version() {
  local version="$1"

  [ -x "$RELEASES_DIR/$version/$APP_NAME" ] || die "release $version is not on this server"
  check_socket_dir

  write_env
  write_unit

  # Replace the symlink atomically: `current` never points nowhere.
  ln -sfn "releases/$version" "$CURRENT_LINK.tmp"
  mv -T "$CURRENT_LINK.tmp" "$CURRENT_LINK"

  systemctl --user restart "$SERVICE_NAME"
  echo "started $SERVICE_NAME on $version"
}

# Ask the running app for /health with its own `healthcheck`, through the socket
# nginx uses. It prints what /health answers: the database and how many focuses are
# in it, which is what says whether the catalog still needs seeding.
read_health() {
  [ -x "$CURRENT_LINK/$APP_NAME" ] \
    && SOCKET_PATH="$SOCKET_PATH" "$CURRENT_LINK/$APP_NAME" healthcheck 2>/dev/null
}

# Poll until the app is actually answering (a fresh start needs a moment, and it
# applies pending migrations first). Returns non-zero if it never does.
wait_for_health() {
  local attempts="${1:-15}" answer
  for _ in $(seq 1 "$attempts"); do
    if answer="$(read_health)"; then
      echo "health check ok (/health on $SOCKET_PATH)"
      echo "    $answer"
      if probe_nginx_view; then
        echo "socket visible from $NGINX_CONTAINER ($NGINX_SOCKET_DIR/$SOCKET_NAME)"
      else
        echo "WARNING: $NGINX_CONTAINER does not see $NGINX_SOCKET_DIR/$SOCKET_NAME" >&2
      fi
      return 0
    fi
    sleep 2
  done
  echo "health check FAILED (/health on $SOCKET_PATH)" >&2
  journalctl --user -u "$SERVICE_NAME" -n 40 --no-pager >&2 || true
  return 1
}

# An empty catalog answers 200 — the app is up, it just has nothing to offer. Say so
# rather than letting a healthy-looking deploy serve a blank page.
warn_if_empty_catalog() {
  case "$(read_health || true)" in
    *'"focuses":0'*|*'"technologies":0'*|*'"staff":0'*)
      echo "warning: a catalog is empty — seed it with: make seed" >&2 ;;
  esac
}

# --- subcommands -----------------------------------------------------------

# Switch to the freshly uploaded release and record the version swap.
cmd_activate() {
  local version="${1:?activate needs a version}"
  local binary="$RELEASES_DIR/$version/$APP_NAME"

  [ -f "$binary" ] || die "$binary not found"
  chmod +x "$binary"

  # Version bookkeeping happens *before* the restart: if the new version fails to
  # come up, previous_version.txt already points at what to roll back to.
  local current
  current="$(read_version "$CURRENT_FILE")"
  if [ -n "$current" ]; then          # absent on the very first deploy
    echo "$current" > "$PREVIOUS_FILE"
  fi
  echo "$version" > "$CURRENT_FILE"

  start_version "$version"
  wait_for_health
  warn_if_empty_catalog
}

# Swap back to previous_version.txt. The two version files trade places, so a second
# rollback returns to where you started. It starts the previous binary on the schema
# as it now is: fine for additive migrations.
cmd_rollback() {
  local current previous
  current="$(read_version "$CURRENT_FILE")"
  previous="$(read_version "$PREVIOUS_FILE")"

  [ -n "$previous" ] || die "no previous version recorded — nothing to roll back to"
  if [ "$previous" = "$current" ]; then
    die "previous version is the current one ($current)"
  fi

  start_version "$previous"

  echo "$previous" > "$CURRENT_FILE"
  if [ -n "$current" ]; then
    echo "$current" > "$PREVIOUS_FILE"
  else
    rm -f "$PREVIOUS_FILE"
  fi

  echo "rolled back to $previous (was $current)"
  wait_for_health
}

# Fill the catalogs from the JSON in the current release. The saved wiki pages are not
# shipped, so this is --from-json only. It replaces the `focuses`, `technologies` and
# `staff` tables wholesale and leaves the `plans` table alone.
cmd_seed() {
  [ -x "$RUN_SCRIPT" ] || die "$RUN_SCRIPT is missing — deploy first"
  "$RUN_SCRIPT" import-focuses --from-json
  "$RUN_SCRIPT" import-technologies --from-json
  "$RUN_SCRIPT" import-staff --from-json
  echo "    $(read_health || echo 'the app does not answer')"
}

# Keep the $KEEP_RELEASES newest releases; never touch what is running or what
# rollback needs.
cmd_prune() {
  local keep="$KEEP_RELEASES" current previous kept=0
  current="$(read_version "$CURRENT_FILE")"
  previous="$(read_version "$PREVIOUS_FILE")"

  # Versions are UTC timestamps: newest first is reverse name order.
  for dir in $(ls -1d "$RELEASES_DIR"/*/ 2>/dev/null | sort -r); do
    local version
    version="$(basename "$dir")"
    kept=$((kept + 1))
    if [ "$kept" -le "$keep" ] || [ "$version" = "$current" ] || [ "$version" = "$previous" ]; then
      continue
    fi
    echo "pruning $version"
    find "$RELEASES_DIR/$version" -type f -delete
    find "$RELEASES_DIR/$version" -depth -type d -empty -delete
  done
}

cmd_status() {
  echo "current : $(read_version "$CURRENT_FILE")"
  echo "previous: $(read_version "$PREVIOUS_FILE")"
  echo
  systemctl --user status "$SERVICE_NAME" --no-pager --lines 0 || true
  echo
  echo "prefix  : ${URL_PREFIX%/}/"
  echo "socket  : $SOCKET_PATH -> $NGINX_CONTAINER:$NGINX_SOCKET_DIR/$SOCKET_NAME"
  local answer
  if answer="$(read_health)"; then
    echo "health  : ok"
    echo "          $answer"
  else
    echo "health  : NOT answering"
  fi
  if probe_nginx_view; then
    echo "nginx   : sees the socket"
  else
    echo "nginx   : does NOT see the socket"
  fi
}

cmd_versions() {
  echo "current : $(read_version "$CURRENT_FILE")"
  echo "previous: $(read_version "$PREVIOUS_FILE")"
  echo "releases:"
  ls -1 "$RELEASES_DIR" 2>/dev/null | sort -r | sed 's/^/  /' || echo "  (none)"
}

# One-time server preparation: directory layout, and a systemd user manager that
# keeps running when nobody is logged in.
cmd_init() {
  mkdir -p "$RELEASES_DIR" "$DB_DIR"
  echo "layout ready under $REMOTE_DIR"

  # Without lingering, user units stop at logout and do not start at boot.
  if [ "$(loginctl show-user "$(id -un)" --property=Linger --value 2>/dev/null)" = yes ]; then
    echo "lingering already enabled for $(id -un)"
  elif sudo -n loginctl enable-linger "$(id -un)" 2>/dev/null || loginctl enable-linger "$(id -un)" 2>/dev/null; then
    echo "lingering enabled for $(id -un)"
  else
    echo "WARNING: could not enable lingering; the app would stop at logout. Run once:" >&2
    echo "  sudo loginctl enable-linger $(id -un)" >&2
  fi

  systemctl --user show-environment >/dev/null 2>&1 \
    || die "no systemd user manager for $(id -un) (systemctl --user fails)"
  echo "systemd user manager ok"

  # Fail here rather than at the first deploy.
  check_socket_dir
  echo "$SOCKET_DIR is mounted in $NGINX_CONTAINER on $NGINX_SOCKET_DIR"
}

# Validate the snippet setup_server.sh just uploaded, then reload nginx.
#
# `nginx -t` FIRST, and remove the file if it fails: this proxy fronts other sites,
# and a broken include would take them all down at its next restart. Reload rather
# than restart: the other sites keep their connections.
cmd_nginx_reload() {
  local conf="$NGINX_CONF_DIR/$NGINX_CONF_NAME"

  docker container inspect "$NGINX_CONTAINER" >/dev/null 2>&1 \
    || die "no container named '$NGINX_CONTAINER' — is the proxy running?"
  [ -f "$conf" ] || die "$conf was not uploaded"

  echo "==> Validating nginx config"
  if ! docker exec "$NGINX_CONTAINER" nginx -t; then
    rm -f "$conf"
    die "nginx refused the config — removed $conf, nginx left untouched"
  fi

  echo "==> Reloading $NGINX_CONTAINER"
  docker exec "$NGINX_CONTAINER" nginx -s reload
  echo "$NGINX_CONTAINER reloaded with $conf"
}

command="${1:-}"
shift || true
case "$command" in
  init)         cmd_init "$@" ;;
  nginx_reload) cmd_nginx_reload "$@" ;;
  activate)     cmd_activate "$@" ;;
  rollback)     cmd_rollback "$@" ;;
  seed)         cmd_seed "$@" ;;
  prune)        cmd_prune "$@" ;;
  status)       cmd_status "$@" ;;
  health)       wait_for_health "${1:-1}" ;;
  versions)     cmd_versions "$@" ;;
  *)            die "unknown command '$command'" \
                    "(init|nginx_reload|activate|rollback|seed|prune|status|health|versions)" ;;
esac
