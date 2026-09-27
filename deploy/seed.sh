#!/usr/bin/env bash
# Fill the server's catalog: make seed
#
# The plans live in the server's MongoDB and survive every deploy; the `focuses`
# collection does not come with the database, so it has to be seeded once — and
# again whenever a re-parsed data/focuses.json ships a changed catalog.
#
# It runs inside the deployed container (the JSON is baked into the image) and
# only ever replaces `focuses`. Saved plans are left alone.
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/config.sh"

echo "==> Seeding the catalog on $SSH_HOST from the image's data/focuses.json"
upload_remote_script
remote seed
