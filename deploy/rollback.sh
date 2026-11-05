#!/usr/bin/env bash
# Go back to the previously deployed version, in one command: make rollback
#
# Purely server-side: the previous release is still there, so this builds and
# transfers nothing. The two version files trade places, which means running it twice
# returns you to where you started.
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/config.sh"

echo "==> Rolling back on $SSH_HOST:$REMOTE_DIR"
# Uploaded every time so deploy and rollback start the app the same way.
upload_remote_script
remote versions
echo

if ! remote rollback; then
  echo >&2
  echo "error: rollback failed — check 'make status' and 'make logs'" >&2
  exit 1
fi

echo
remote versions
