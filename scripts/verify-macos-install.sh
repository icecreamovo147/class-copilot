#!/usr/bin/env bash

set -euo pipefail

DMG_PATH="${1:-src-tauri/target/release/bundle/dmg/数字化班级事务管理系统_1.0.0_aarch64.dmg}"
EXPECTED_ID="${EXPECTED_ID:-com.class-copilot}"

if [[ ! -f "$DMG_PATH" ]]; then
  echo "dmg not found: $DMG_PATH" >&2
  exit 1
fi

WORK_DIR="$(mktemp -d)"
INSTALL_ROOT="$WORK_DIR/Applications"
mkdir -p "$INSTALL_ROOT"

cleanup() {
  if mount | grep -q "$WORK_DIR/mount"; then
    hdiutil detach "$WORK_DIR/mount" >/dev/null 2>&1 || true
  fi
  rm -rf "$WORK_DIR"
}
trap cleanup EXIT

mkdir -p "$WORK_DIR/mount"
hdiutil attach "$DMG_PATH" -mountpoint "$WORK_DIR/mount" -nobrowse -quiet

APP_PATH="$(find "$WORK_DIR/mount" -maxdepth 1 -type d -name '*.app' | head -n 1)"
if [[ -z "$APP_PATH" ]]; then
  echo "no app bundle found in dmg" >&2
  exit 1
fi

cp -R "$APP_PATH" "$INSTALL_ROOT/"
INSTALLED_APP="$INSTALL_ROOT/$(basename "$APP_PATH")"
INSTALLED_PLIST="$INSTALLED_APP/Contents/Info.plist"
ACTUAL_ID="$(plutil -extract CFBundleIdentifier raw -o - "$INSTALLED_PLIST")"

if [[ "$ACTUAL_ID" != "$EXPECTED_ID" ]]; then
  echo "bundle id mismatch: expected $EXPECTED_ID, got $ACTUAL_ID" >&2
  exit 1
fi

EXECUTABLE_PATH="$INSTALLED_APP/Contents/MacOS/class-copilot"
if [[ ! -x "$EXECUTABLE_PATH" ]]; then
  echo "executable not found: $EXECUTABLE_PATH" >&2
  exit 1
fi

echo "verified installable app: $INSTALLED_APP"

RUNTIME_HOME="$WORK_DIR/runtime-home"
mkdir -p "$RUNTIME_HOME"
LEGACY_DATA_ROOT="$RUNTIME_HOME/Library/Application Support/com.class-copilot.app"
mkdir -p "$LEGACY_DATA_ROOT/attachments/homework"
printf "legacy-file" > "$LEGACY_DATA_ROOT/attachments/homework/migration.txt"
START_LOG="$WORK_DIR/start.log"
START_ERR="$WORK_DIR/start.err"
HOME="$RUNTIME_HOME" "$EXECUTABLE_PATH" >"$START_LOG" 2>"$START_ERR" &
APP_PID=$!
sleep 8
if ! kill -0 "$APP_PID" >/dev/null 2>&1; then
  echo "app exited unexpectedly during startup smoke test" >&2
  cat "$START_ERR" >&2 || true
  exit 1
fi
kill "$APP_PID" >/dev/null 2>&1 || true
wait "$APP_PID" || true

DATA_ROOT="$RUNTIME_HOME/Library/Application Support/$EXPECTED_ID"
if [[ ! -d "$DATA_ROOT/logs" ]]; then
  echo "startup smoke test did not create logs directory: $DATA_ROOT/logs" >&2
  exit 1
fi
if [[ ! -f "$DATA_ROOT/class_management.db" ]]; then
  echo "startup smoke test did not create database: $DATA_ROOT/class_management.db" >&2
  exit 1
fi
if [[ ! -f "$DATA_ROOT/attachments/homework/migration.txt" ]]; then
  echo "legacy data migration did not preserve attachment marker" >&2
  exit 1
fi

echo "verified startup data directories: $DATA_ROOT"
rm -rf "$INSTALLED_APP"
if [[ -e "$INSTALLED_APP" ]]; then
  echo "app uninstall cleanup failed" >&2
  exit 1
fi

echo "verified uninstall cleanup: $INSTALLED_APP"
