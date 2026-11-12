#!/usr/bin/env bash
# Build here, ship the release over SSH, restart there. Run it with: make deploy
#
# Steps: cross-compile (version = UTC timestamp) -> scp the binary, static/ and the
# catalogs' JSON -> version bookkeeping -> restart the unit -> health check -> prune.
#
# The database is never shipped or touched: the app applies pending migrations when
# it starts, and the plans in it survive every deploy.
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/config.sh"

# UTC so two machines deploying the same day still sort chronologically.
VERSION="$(date -u +%Y%m%d-%H%M%S)"
BINARY="$REPO_ROOT/target/$TARGET/release/$APP_NAME"

echo "==> Deploying $APP_NAME $VERSION to $SSH_HOST:$REMOTE_DIR"

# 1. Cross-compile for the SERVER: zig provides the linker and the musl libc, so a
#    Mac builds a static Linux binary without any VM or container.
if ! cargo zigbuild --help >/dev/null 2>&1; then
  echo "error: cargo-zigbuild is needed to build for $TARGET. Install it once with:" >&2
  echo "       brew install zig && cargo install cargo-zigbuild && rustup target add $TARGET" >&2
  exit 1
fi
echo "==> Building for $TARGET"
cargo zigbuild --release --locked --target "$TARGET" --manifest-path "$REPO_ROOT/Cargo.toml"

# Confirm what actually came out before shipping it.
expected_arch="${TARGET%%-*}"
case "$expected_arch" in x86_64) expected_arch="x86-64" ;; aarch64) expected_arch="aarch64" ;; esac
file "$BINARY" | grep -q "ELF.*$expected_arch" \
  || { echo "error: $BINARY is not a $expected_arch Linux binary:" >&2; file "$BINARY" >&2; exit 1; }
du -h "$BINARY" | cut -f1 | xargs echo "    binary size:"

# 2. Ship it. remote.sh goes first so the server side always matches this checkout.
#    A release holds what the binary serves or seeds from, and the saved wiki pages
#    stay on the laptop: re-parsing is a laptop job, and its result is committed.
echo "==> Uploading"
upload_remote_script
ssh "$SSH_HOST" "mkdir -p '$REMOTE_DIR/releases/$VERSION/data'"
scp -C "$BINARY" "$SSH_HOST:$REMOTE_DIR/releases/$VERSION/$APP_NAME"
scp -Cqr "$REPO_ROOT/static" "$SSH_HOST:$REMOTE_DIR/releases/$VERSION/static"
scp -Cq "$REPO_ROOT/data/focuses.json" "$REPO_ROOT/data/technologies.json" "$REPO_ROOT/data/staff.json" \
  "$SSH_HOST:$REMOTE_DIR/releases/$VERSION/data/"

# 3. Record the version swap, restart, wait for /health. Any failure here stops the
#    script (set -e) with the old version still recorded as previous, so
#    `make rollback` is the immediate way out.
echo "==> Activating $VERSION on the server"
if ! remote activate "$VERSION"; then
  echo >&2
  echo "error: $VERSION did not come up. Roll back with: make rollback" >&2
  exit 1
fi

# 4. Housekeeping — only after a healthy start, so a failed deploy never deletes
#    the release we might need to go back to.
echo "==> Pruning old releases (keeping $KEEP_RELEASES)"
remote prune

echo
echo "Deployed $APP_NAME $VERSION"
remote versions
