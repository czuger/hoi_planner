#!/usr/bin/env bash
# Read-only looks at what runs on the server: make logs | status | versions
#
# Lives here rather than in the Makefile so it reads the same deploy/config.sh as
# every other script, instead of a second copy of the host, directory and unit name.
#
# Usage: deploy/server.sh <logs | status | versions>
set -euo pipefail

command="${1:-}"

source "$(dirname "${BASH_SOURCE[0]}")/config.sh"

case "$command" in
  logs)
    ssh -t "$SSH_HOST" "journalctl --user -u '$SERVICE_NAME' -f -n 100"
    ;;
  status|versions)
    # The server's copy may predate this checkout (or not exist yet, before the
    # first deploy); status/versions only read, so refresh it.
    upload_remote_script
    remote "$command"
    ;;
  *)
    echo "usage: $0 <logs|status|versions>" >&2
    exit 1
    ;;
esac
