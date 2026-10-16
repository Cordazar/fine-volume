#!/bin/sh
# Stop the LaunchAgent and remove what install.sh put in place.
set -eu

label=io.github.cordazar.fine-volume
launchctl bootout "gui/$(id -u)/$label" 2>/dev/null || true
rm -f "$HOME/Library/LaunchAgents/$label.plist" "$HOME/.local/bin/fine-volume"
echo "removed $label; you can also remove fine-volume from Accessibility in System Settings"
