#!/usr/bin/env bash
# Install pomodoro for the current user (no sudo needed).
set -euo pipefail

cd "$(dirname "$0")"

BIN_DIR="$HOME/.local/bin"
ICON_DIR="$HOME/.local/share/icons/hicolor/256x256/apps"
APPS_DIR="$HOME/.local/share/applications"

echo "Building release binary..."
cargo build --release

mkdir -p "$BIN_DIR" "$ICON_DIR" "$APPS_DIR"

echo "Installing binary to $BIN_DIR/pomodoro"
install -m 0755 target/release/pomodoro "$BIN_DIR/pomodoro"

echo "Installing icon to $ICON_DIR/pomodoro.png"
install -m 0644 assets/pomodoro.png "$ICON_DIR/pomodoro.png"

# Materialize the desktop entry from the template, substituting the
# absolute binary path. Using an absolute path avoids "command not found"
# when the graphical session doesn't pick up ~/.local/bin from .bashrc.
DESKTOP_SRC="assets/pomodoro.desktop.in"
DESKTOP_DST="$APPS_DIR/pomodoro.desktop"

if [[ ! -f "$DESKTOP_SRC" ]]; then
  echo "Error: $DESKTOP_SRC not found." >&2
  echo "Make sure the assets/ directory contains pomodoro.desktop.in." >&2
  exit 1
fi

echo "Installing desktop entry to $DESKTOP_DST"
# Escape the bin path for sed (handles spaces; rejects '|' in path).
ESCAPED_BIN="$(printf '%s\n' "$BIN_DIR/pomodoro" | sed -e 's/[\/&|]/\\&/g')"
sed "s|@BIN@|$ESCAPED_BIN|g" "$DESKTOP_SRC" > "$DESKTOP_DST"
chmod 0644 "$DESKTOP_DST"

# Validate the desktop file if the tool is available.
if command -v desktop-file-validate >/dev/null; then
  if ! desktop-file-validate "$DESKTOP_DST"; then
    echo "Warning: desktop-file-validate reported issues with $DESKTOP_DST" >&2
  fi
fi

# Refresh caches so the icon shows up immediately.
if command -v update-desktop-database >/dev/null; then
  update-desktop-database "$APPS_DIR" || true
fi
if command -v gtk-update-icon-cache >/dev/null; then
  gtk-update-icon-cache -f "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
fi

echo
echo "Installed. Make sure $BIN_DIR is on your PATH (for terminal use)."
echo "Launch via your application menu (search 'Pomodoro') or run: pomodoro"
echo
echo "If the dock icon is still wrong or generic after launch:"
echo "  1. Confirm the window's app id matches: it should be 'pomodoro'."
echo "  2. Verify the desktop file:"
echo "       desktop-file-validate $DESKTOP_DST"
echo "  3. Test launch through the desktop entry directly:"
echo "       gtk-launch pomodoro"
echo "  4. Wayland caches app identities aggressively — log out and back in."
echo "  5. Check session type:  echo \$XDG_SESSION_TYPE"
