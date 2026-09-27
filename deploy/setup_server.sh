#!/usr/bin/env bash
# ONE-TIME server preparation. Run from your laptop: make setup
#
# Creates the directory layout, checks Docker is usable over SSH, and tells you
# which config file you still have to copy by hand. It never touches config/ if
# it already exists — that is the whole point of the code/config split.
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/config.sh"

echo "==> Target: $SSH_HOST:$REMOTE_DIR"

# 1. Docker must be there, and usable by this SSH user without sudo.
echo "==> Checking Docker on the server"
if ! ssh "$SSH_HOST" 'command -v docker >/dev/null'; then
  echo "error: docker is not installed on $SSH_HOST" >&2
  exit 1
fi
if ! ssh "$SSH_HOST" 'docker info >/dev/null 2>&1'; then
  echo "error: 'docker info' fails on $SSH_HOST — add the SSH user to the docker group:" >&2
  echo "       sudo usermod -aG docker \$USER   (then log out and back in)" >&2
  exit 1
fi
ssh "$SSH_HOST" 'docker --version'

# 2. Directory layout: releases/ (tarballs) and config/ (the .env, ours to never
#    touch). No data/ — the container writes nothing, the plans are in MongoDB.
echo "==> Creating $REMOTE_DIR/{releases,config}"
ssh "$SSH_HOST" "mkdir -p '$REMOTE_DIR'"
upload_remote_script
remote init

# 3. The nginx snippet: rendered from deploy/nginx/, dropped in the proxy's config
#    directory, validated there, and only then does nginx restart. Setup-only —
#    routing does not change from one deploy to the next.
echo "==> Installing the nginx config ($NGINX_CONF_NAME -> $NGINX_CONF_DIR)"
install_nginx_conf

# 4. What is still missing before the first deploy.
echo "==> Checking server config"
missing=0
if ssh "$SSH_HOST" "test -f '$REMOTE_DIR/config/.env'"; then
  echo "    ok      config/.env"
else
  echo "    MISSING config/.env"
  missing=1
fi

cat <<EOF2

Layout ready on $SSH_HOST:$REMOTE_DIR
  releases/   deployed .tar images (last $KEEP_RELEASES kept)
  config/     .env — server-owned, no deploy ever writes here

The app publishes no port. It joins nginx's shared '$DOCKER_NETWORK' network and
answers there at http://$CONTAINER_NAME:$APP_PORT, which is what
$NGINX_CONF_DIR/$NGINX_CONF_NAME proxies ${URL_PREFIX%/}/ to.

EOF2

if [ "$missing" -eq 1 ]; then
  cat <<EOF2
Still to do, once, by hand:

  scp .env $SSH_HOST:$REMOTE_DIR/config/.env

In that server-side .env, remember to:
  - set APP_ENV=prod        (the database is then hoi4_planner_prod)
  - set URL_PREFIX=${URL_PREFIX%/}
    (must match the nginx snippet, or every link lands outside ${URL_PREFIX%/}/)
  - set FLASK_SECRET_KEY to a real random value
    (python -c "import secrets; print(secrets.token_hex(32))")
  - point MONGO_URI at the host's MongoDB as seen from inside the container:
    MONGO_URI=mongodb://host.docker.internal:27017
    (the container is started with --add-host=host.docker.internal:host-gateway;
     mongod must listen on the docker bridge, not on 127.0.0.1 only)

EOF2
fi

echo "Next: make deploy, then make seed — the catalog is not in the database yet."
echo "      The site then answers at https://<your-host>${URL_PREFIX%/}"
