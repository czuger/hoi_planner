#!/usr/bin/env bash
# Fill the server's catalogs: make seed
#
# The plans live in the server's SQLite file and survive every deploy; the `focuses`,
# `technologies` and `staff` tables have to be seeded once, and again whenever a
# re-parsed JSON file ships a changed catalog. It reads the JSON of the current release
# and only ever replaces those three tables. Saved plans are left alone.
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/config.sh"

echo "==> Seeding the catalogs on $SSH_HOST from the current release's JSON"
upload_remote_script
remote seed
