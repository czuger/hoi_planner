#!/usr/bin/env bash
# Shared settings for the deploy scripts, sourced by all of them (never run alone).
#
# Every value can be overridden from the environment without editing this file:
#   SSH_HOST=ced@other-box make deploy
#
# This file is versioned on purpose: it describes *where* the app goes, which is
# not a secret. The only secret this app has is FLASK_SECRET_KEY, and it lives
# only in $REMOTE_DIR/config/.env on the server.

# Where to deploy.
SSH_HOST="${SSH_HOST:-ced@nuc150}"
REMOTE_DIR="${REMOTE_DIR:-/home/ced/python/hoi4_planner}"

# What runs there. IMAGE_NAME is also the local build tag, the version is appended.
CONTAINER_NAME="${CONTAINER_NAME:-app-hoi4-planner}"
IMAGE_NAME="${IMAGE_NAME:-hoi4-planner}"

# The port gunicorn listens on INSIDE the container. It is never published to the
# host — the app is reached over the docker network below, as
# http://$CONTAINER_NAME:$APP_PORT. Changing this means changing the gunicorn
# --bind in the Dockerfile too.
APP_PORT="${APP_PORT:-8000}"

# The shared docker network nginx sits on. It belongs to nginx, not to us: the
# scripts only join it and refuse to run if it is missing (creating one nginx is
# not attached to would look fine and 502 at request time).
DOCKER_NETWORK="${DOCKER_NETWORK:-nginx-common-network}"

# The dockerized nginx: its container, and the directory its config includes from.
# `make setup` / `make nginx` render the snippet below into that directory and
# restart the container; a deploy never touches either.
NGINX_CONTAINER="${NGINX_CONTAINER:-nginx-proxy}"
NGINX_CONF_DIR="${NGINX_CONF_DIR:-/home/ced/services/nginx_proxy/sites/apps}"
NGINX_CONF_NAME="${NGINX_CONF_NAME:-hoi4_planner.conf}"

# Sub-path the site is served under, i.e. https://<host>$URL_PREFIX. Used ONLY to
# render the nginx snippet.
#
# The app needs the same value as URL_PREFIX in the server's config/.env, so that
# it builds its URLs under the prefix — and config/.env is server-owned, which no
# deploy step may read or write. Nothing here can check the two agree; setup prints
# it as a reminder. Change the prefix => change it in both places.
URL_PREFIX="${URL_PREFIX:-/hoi4}"

# Architecture of the SERVER, not of your laptop. Building on an Apple Silicon Mac
# defaults to linux/arm64, which the x86_64 server can only run (badly) under
# emulation — hence pinning it here. deploy.sh checks the built image matches.
# On an ARM server (a Pi, an ARM VPS): TARGET_PLATFORM=linux/arm64 make deploy
TARGET_PLATFORM="${TARGET_PLATFORM:-linux/amd64}"

# How many releases (.tar + loaded image) stay on the server. 3 means "current,
# previous, and one more" — rollback only ever needs the previous one.
KEEP_RELEASES="${KEEP_RELEASES:-3}"

# Everything below is derived; no need to touch it.
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REMOTE_SCRIPT="$REMOTE_DIR/remote.sh"

# The server side of every script lives in deploy/remote.sh and is uploaded before
# each run, so deploy and rollback can never drift apart on `docker run` options.
upload_remote_script() {
  scp -q "$REPO_ROOT/deploy/remote.sh" "$SSH_HOST:$REMOTE_SCRIPT"
  ssh "$SSH_HOST" "chmod +x '$REMOTE_SCRIPT'"
}

# Render the nginx snippet from deploy/nginx/, ship it, and have the server
# validate it before restarting nginx. Used by setup_server.sh and `make nginx`;
# no deploy ever calls it — routing does not change between versions.
install_nginx_conf() {
  local template="$REPO_ROOT/deploy/nginx/$NGINX_CONF_NAME.template"
  local rendered="$REPO_ROOT/dist/$NGINX_CONF_NAME"

  [ -f "$template" ] || { echo "error: $template not found" >&2; return 1; }

  mkdir -p "$REPO_ROOT/dist"
  # nginx variable names take [A-Za-z0-9_] only, so the container name cannot be
  # used as-is (app-hoi4-planner -> app_hoi4_planner).
  local container_var="${CONTAINER_NAME//[^a-zA-Z0-9]/_}"
  # Trailing slash trimmed so "/hoi4/" and "/hoi4" both render the same block.
  sed -e "s|__PREFIX__|${URL_PREFIX%/}|g" \
      -e "s|__CONTAINER_VAR__|$container_var|g" \
      -e "s|__CONTAINER__|$CONTAINER_NAME|g" \
      -e "s|__APP_PORT__|$APP_PORT|g" \
      "$template" > "$rendered"

  ssh "$SSH_HOST" "mkdir -p '$NGINX_CONF_DIR'"
  scp -q "$rendered" "$SSH_HOST:$NGINX_CONF_DIR/$NGINX_CONF_NAME"
  rm -f "$rendered"

  upload_remote_script
  remote nginx_reload
}

# Runs remote.sh on the server with this file's settings passed through, so the
# server never keeps its own stale copy of them.
remote() {
  ssh "$SSH_HOST" \
    "REMOTE_DIR='$REMOTE_DIR' CONTAINER_NAME='$CONTAINER_NAME' IMAGE_NAME='$IMAGE_NAME' \
     APP_PORT='$APP_PORT' DOCKER_NETWORK='$DOCKER_NETWORK' KEEP_RELEASES='$KEEP_RELEASES' \
     NGINX_CONTAINER='$NGINX_CONTAINER' NGINX_CONF_DIR='$NGINX_CONF_DIR' \
     NGINX_CONF_NAME='$NGINX_CONF_NAME' \
     bash '$REMOTE_SCRIPT' $*"
}
