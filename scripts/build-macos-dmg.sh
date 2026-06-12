#!/usr/bin/env bash

set -euo pipefail

APP_PATH="${1:-src-tauri/target/release/bundle/macos/数字化班级事务管理系统.app}"
DMG_PATH="${2:-src-tauri/target/release/bundle/dmg/数字化班级事务管理系统_1.0.0_aarch64.dmg}"
VOLUME_NAME="${VOLUME_NAME:-数字化班级事务管理系统}"

if [[ ! -d "$APP_PATH" ]]; then
  echo "app bundle not found: $APP_PATH" >&2
  exit 1
fi

mkdir -p "$(dirname "$DMG_PATH")"

STAGING_DIR="$(mktemp -d)"
cleanup() {
  rm -rf "$STAGING_DIR"
}
trap cleanup EXIT

cp -R "$APP_PATH" "$STAGING_DIR/"
ln -s /Applications "$STAGING_DIR/Applications"
rm -f "$DMG_PATH"

hdiutil create \
  -volname "$VOLUME_NAME" \
  -srcfolder "$STAGING_DIR" \
  -ov \
  -format UDZO \
  "$DMG_PATH"
