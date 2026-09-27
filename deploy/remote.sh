#!/usr/bin/env bash
# THE SERVER SIDE of the deploy. Never run this from your laptop.
#
# deploy.sh, rollback.sh and seed.sh upload this file before every run and then
# call one of its subcommands over SSH. It exists so that the `docker run` line
# lives in exactly one place: a rollback must relaunch the app with the same
# mounts, network and restart policy as a deploy, and duplicating those flags is
# how they drift.
#
# Usage: bash remote.sh <init | nginx_reload | activate VERSION | rollback | seed
#                        | prune | status | health | versions>
set -euo pipefail

# Defaults mirror deploy/config.sh; the caller normally overrides them via env.
REMOTE_DIR="${REMOTE_DIR:-/home/ced/python/hoi4_planner}"
CONTAINER_NAME="${CONTAINER_NAME:-app-hoi4-planner}"
IMAGE_NAME="${IMAGE_NAME:-hoi4-planner}"
APP_PORT="${APP_PORT:-8000}"
DOCKER_NETWORK="${DOCKER_NETWORK:-nginx-common-network}"
KEEP_RELEASES="${KEEP_RELEASES:-3}"
NGINX_CONTAINER="${NGINX_CONTAINER:-nginx-proxy}"
NGINX_CONF_DIR="${NGINX_CONF_DIR:-/home/ced/services/nginx_proxy/sites/apps}"
NGINX_CONF_NAME="${NGINX_CONF_NAME:-hoi4_planner.conf}"

RELEASES_DIR="$REMOTE_DIR/releases"
CONFIG_DIR="$REMOTE_DIR/config"
CURRENT_FILE="$REMOTE_DIR/current_version.txt"
PREVIOUS_FILE="$REMOTE_DIR/previous_version.txt"

die() { echo "error: $*" >&2; exit 1; }

read_version() { [ -f "$1" ] && cat "$1" || true; }

# --- network ---------------------------------------------------------------

# The network belongs to nginx and is shared by every app it fronts — we only
# join it. Deliberately NOT created here: a network we invented would exist,
# accept the container, and 502 every request because nginx is not on it.
check_network() {
  docker network inspect "$DOCKER_NETWORK" >/dev/null 2>&1 || die \
    "docker network '$DOCKER_NETWORK' does not exist — it is nginx's. Existing ones:
$(docker network ls --format '  {{.Name}}')"
}

# --- container -------------------------------------------------------------

# Start (or restart) the app on a given version. This is the single source of
# truth for how the container is run.
#
# The container writes nothing: plans live in the host's MongoDB and the catalog
# is seeded into it, so there is no volume here beyond the read-only .env.
start_container() {
  local version="$1"

  docker image inspect "$IMAGE_NAME:$version" >/dev/null 2>&1 \
    || die "image $IMAGE_NAME:$version is not loaded on this server"

  [ -f "$CONFIG_DIR/.env" ] || die "$CONFIG_DIR/.env is missing — scp it there first"

  check_network

  # -f: also covers a container left in a stopped/created state.
  docker rm -f "$CONTAINER_NAME" >/dev/null 2>&1 || true

  docker run -d \
    --name "$CONTAINER_NAME" \
    --restart unless-stopped \
    `# No published port at all: on this network the app answers at` \
    `# http://$CONTAINER_NAME:$APP_PORT through docker's internal DNS, and` \
    `# nothing at all is bound on the host.` \
    --network "$DOCKER_NETWORK" \
    `# MongoDB runs on the host, so the container needs a name for the host itself.` \
    --add-host=host.docker.internal:host-gateway \
    `# Config is the server's, read-only, and never part of the image.` \
    -v "$CONFIG_DIR/.env:/app/.env:ro" \
    "$IMAGE_NAME:$version" >/dev/null

  echo "started $CONTAINER_NAME on $IMAGE_NAME:$version"
}

# Hit /health from *inside* the container. Nothing is published on the host, so
# the probe cannot come from outside; `docker exec` is the way in, and python is
# already there (the slim image has no curl).
probe_health() {
  docker exec "$CONTAINER_NAME" python -c \
    "import urllib.request; urllib.request.urlopen('http://127.0.0.1:$APP_PORT/health', timeout=3)" \
    >/dev/null 2>&1
}

# What /health answers, verbatim — it carries the database name and how many
# focuses are in it, which is what says whether the catalog still needs seeding.
read_health() {
  docker exec "$CONTAINER_NAME" python -c \
    "import urllib.request; print(urllib.request.urlopen('http://127.0.0.1:$APP_PORT/health', timeout=3).read().decode())" \
    2>/dev/null || true
}

# Poll until gunicorn is actually answering (a fresh container needs a second or
# two). Returns non-zero if it never does.
wait_for_health() {
  local attempts="${1:-15}"
  for _ in $(seq 1 "$attempts"); do
    if probe_health; then
      echo "health check ok (/health inside $CONTAINER_NAME)"
      echo "    $(read_health)"
      return 0
    fi
    sleep 2
  done
  echo "health check FAILED (/health inside $CONTAINER_NAME)" >&2
  docker logs --tail 40 "$CONTAINER_NAME" >&2 || true
  return 1
}

# An empty catalog answers 200 — the app is up, it just has nothing to offer. Say
# so rather than letting a healthy-looking deploy serve a blank page.
warn_if_empty_catalog() {
  case "$(read_health)" in
    *'"focuses": 0'*|*'"focuses":0'*)
      echo "warning: the catalog is empty — seed it with: make seed" >&2 ;;
  esac
}

# --- subcommands -----------------------------------------------------------

# Load the freshly uploaded .tar, switch to it, and record the version swap.
cmd_activate() {
  local version="${1:?activate needs a version}"
  local tarball="$RELEASES_DIR/$version.tar"

  [ -f "$tarball" ] || die "$tarball not found"
  docker load -i "$tarball"

  # Version bookkeeping happens *before* the restart: if the new container fails
  # to come up, previous_version.txt already points at what to roll back to.
  local current
  current="$(read_version "$CURRENT_FILE")"
  if [ -n "$current" ]; then          # absent on the very first deploy
    echo "$current" > "$PREVIOUS_FILE"
  fi
  echo "$version" > "$CURRENT_FILE"

  start_container "$version"
  wait_for_health
  warn_if_empty_catalog
}

# Swap back to previous_version.txt. The two version files trade places, so a
# second rollback returns to where you started.
cmd_rollback() {
  local current previous
  current="$(read_version "$CURRENT_FILE")"
  previous="$(read_version "$PREVIOUS_FILE")"

  [ -n "$previous" ] || die "no previous version recorded — nothing to roll back to"
  if [ "$previous" = "$current" ]; then
    die "previous version is the current one ($current)"
  fi

  start_container "$previous"

  echo "$previous" > "$CURRENT_FILE"
  if [ -n "$current" ]; then
    echo "$current" > "$PREVIOUS_FILE"
  else
    rm -f "$PREVIOUS_FILE"
  fi

  echo "rolled back to $previous (was $current)"
  wait_for_health
}

# Fill the `focuses` collection from the JSON baked into the image. The saved wiki
# page is not shipped (2 MB the server has no use for), so this is --from-json
# only: re-parsing the page is a laptop job, and its result is committed.
#
# It replaces the catalog wholesale and leaves the `plans` collection alone.
cmd_seed() {
  docker container inspect "$CONTAINER_NAME" >/dev/null 2>&1 \
    || die "$CONTAINER_NAME is not running — deploy first"
  docker exec "$CONTAINER_NAME" python scripts/import_focuses.py --from-json
  echo "    $(read_health)"
}

# Keep the $KEEP_RELEASES newest tarballs and images; never touch what is running
# or what rollback needs.
cmd_prune() {
  local keep="$KEEP_RELEASES" current previous
  current="$(read_version "$CURRENT_FILE")"
  previous="$(read_version "$PREVIOUS_FILE")"

  local kept=0
  # Newest first; anything past the limit goes, unless it is current/previous.
  for tarball in $(ls -1t "$RELEASES_DIR"/*.tar 2>/dev/null); do
    local version
    version="$(basename "$tarball" .tar)"
    kept=$((kept + 1))
    if [ "$kept" -le "$keep" ] || [ "$version" = "$current" ] || [ "$version" = "$previous" ]; then
      continue
    fi
    echo "pruning $version"
    rm -f "$tarball"
    docker image rm "$IMAGE_NAME:$version" >/dev/null 2>&1 || true
  done

  # Images with no tarball left (e.g. loaded by hand) follow the same rule.
  for version in $(docker images --format '{{.Tag}}' "$IMAGE_NAME" | tail -n +$((keep + 1))); do
    if [ "$version" = "$current" ] || [ "$version" = "$previous" ]; then
      continue
    fi
    docker image rm "$IMAGE_NAME:$version" >/dev/null 2>&1 || true
  done
}

cmd_status() {
  echo "current : $(read_version "$CURRENT_FILE")"
  echo "previous: $(read_version "$PREVIOUS_FILE")"
  echo
  # No Ports column: nothing is published, that is the point.
  docker ps --filter "name=^/$CONTAINER_NAME$" \
    --format 'table {{.Names}}\t{{.Image}}\t{{.Status}}'
  echo
  echo "network : $DOCKER_NETWORK -> http://$CONTAINER_NAME:$APP_PORT"
  echo "on it   : $(docker network inspect "$DOCKER_NETWORK" \
    --format '{{range .Containers}}{{.Name}} {{end}}' 2>/dev/null || echo '(no such network)')"
  if probe_health; then
    echo "health  : ok"
    echo "          $(read_health)"
  else
    echo "health  : NOT answering"
  fi
}

cmd_versions() {
  echo "current : $(read_version "$CURRENT_FILE")"
  echo "previous: $(read_version "$PREVIOUS_FILE")"
  echo "releases:"
  ls -1t "$RELEASES_DIR"/*.tar 2>/dev/null | xargs -r -n1 basename || echo "  (none)"
}

# One-time server preparation: the directory layout. There is no data/ here — the
# container writes nothing, everything it keeps is in the host's MongoDB.
cmd_init() {
  mkdir -p "$RELEASES_DIR" "$CONFIG_DIR"
  chmod 700 "$CONFIG_DIR"
  echo "layout ready under $REMOTE_DIR"

  # Fail here rather than at the first deploy if nginx's network is not around.
  check_network
  echo "network '$DOCKER_NETWORK' found"
}

# Validate the snippet setup_server.sh just uploaded, then restart nginx.
#
# `nginx -t` FIRST, and remove the file if it fails: this proxy fronts other
# sites, and a broken include would take them all down at its next restart.
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

  echo "==> Restarting $NGINX_CONTAINER"
  docker restart "$NGINX_CONTAINER" >/dev/null
  echo "$NGINX_CONTAINER restarted with $conf"
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
