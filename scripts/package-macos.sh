#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
APP_PATH=${1:-"$REPO_ROOT/target/OpenStoryRS.app"}
RUST_CARGO_BIN=${RUST_CARGO_BIN:-cargo}

case "$APP_PATH" in
	/*) ;;
	*) APP_PATH="$REPO_ROOT/$APP_PATH" ;;
esac

cd "$REPO_ROOT"
CARGO_PROFILE_RELEASE_STRIP=none "$RUST_CARGO_BIN" build --release -p openstory-client --bin OpenStoryRS
mkdir -p "$APP_PATH/Contents/MacOS" "$APP_PATH/Contents/Resources"
cp target/release/OpenStoryRS "$APP_PATH/Contents/MacOS/OpenStoryRS"
cp packaging/macos/Info.plist "$APP_PATH/Contents/Info.plist"
printf 'APPL????' > "$APP_PATH/Contents/PkgInfo"
/usr/bin/codesign --force --deep --sign - "$APP_PATH"

printf 'Created %s\n' "$APP_PATH"
