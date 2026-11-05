#!/usr/bin/env bash
# ONE-TIME server preparation. Run from your laptop: make setup
#
# Creates the directory layout, checks systemd can run the app as this SSH user and
# that the nginx container sees the socket directory, and installs the nginx snippet.
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/config.sh"

echo "==> Target: $SSH_HOST:$REMOTE_DIR"

# nginx runs in a container: every nginx and socket check goes through docker.
ssh "$SSH_HOST" 'docker info >/dev/null 2>&1' || {
  echo "error: 'docker info' fails on $SSH_HOST — the SSH user must be in the docker group." >&2
  exit 1
}

# 1. Directory layout: releases/ (one directory per version) and data/db/ (the SQLite
#    database, the only thing the app writes). Also checks the systemd user manager,
#    lingering, and that the nginx container mounts the socket directory.
echo "==> Creating $REMOTE_DIR/{releases,data/db}"
ssh "$SSH_HOST" "mkdir -p '$REMOTE_DIR'"
upload_remote_script
remote init

# 2. The nginx snippet: rendered from deploy/nginx/, dropped in nginx's config
#    directory, validated there, and only then does nginx reload. Setup-only —
#    routing does not change from one deploy to the next.
echo "==> Installing the nginx config ($NGINX_CONF_NAME -> $NGINX_CONF_DIR)"
install_nginx_conf

cat <<SUMMARY

Layout ready on $SSH_HOST:$REMOTE_DIR
  releases/   deployed versions (last $KEEP_RELEASES kept); current -> the running one
  data/db/    the SQLite database, which survives every deploy
  app.env     the app's environment, written by each deploy (do not edit)
  run         runs the current binary with that environment

The app runs as the systemd user unit '$SERVICE_NAME' and opens no port: it listens on
$SOCKET_DIR/$SOCKET_NAME, which $NGINX_CONTAINER sees as
$NGINX_SOCKET_DIR/$SOCKET_NAME and serves under ${URL_PREFIX%/}/.

SUMMARY

echo "Next: make deploy, then make seed — the catalog is not in the database yet."
echo "      The site then answers at https://<your-host>${URL_PREFIX%/}"
