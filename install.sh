#!/bin/sh
# Build, sign and (re)load the LaunchAgent.
set -eu
cd "$(dirname "$0")"

label=io.github.cordazar.fine-volume
bin="$HOME/.local/bin/fine-volume"
plist="$HOME/Library/LaunchAgents/$label.plist"

cargo build --release
mkdir -p "$(dirname "$bin")"
cp target/release/fine-volume "$bin"
# A stable identifier keeps the Accessibility grant across rebuilds more often.
codesign --force -s - -i "$label" "$bin"

sed "s|__BIN__|$bin|" "$label.plist" > "$plist"
launchctl bootout "gui/$(id -u)/$label" 2>/dev/null || true
launchctl bootstrap "gui/$(id -u)" "$plist"
echo "loaded $label; log: /tmp/fine-volume.log"
