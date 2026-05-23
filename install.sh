#!/usr/bin/env bash
# Install pomodoro for the current user (no sudo needed).
# Detects the OS and installs appropriately:
#   macOS → ~/Applications/Pomodoro.app bundle
#   Linux → ~/.local/share/applications + ~/.local/bin
set -euo pipefail

cd "$(dirname "$0")"

# ── macOS ─────────────────────────────────────────────────────────────────────

install_macos() {
  APP="$HOME/Applications/Pomodoro.app"
  CONTENTS="$APP/Contents"
  MACOS_DIR="$CONTENTS/MacOS"
  RESOURCES="$CONTENTS/Resources"

  echo "Creating app bundle at $APP ..."
  rm -rf "$APP"
  mkdir -p "$MACOS_DIR" "$RESOURCES"

  echo "Installing binary..."
  install -m 0755 target/release/pomodoro "$MACOS_DIR/pomodoro"

  echo "Writing Info.plist..."
  cat > "$CONTENTS/Info.plist" << 'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>
  <string>Pomodoro</string>
  <key>CFBundleDisplayName</key>
  <string>Pomodoro</string>
  <key>CFBundleIdentifier</key>
  <string>io.pomodoro.app</string>
  <key>CFBundleExecutable</key>
  <string>pomodoro</string>
  <key>CFBundleIconFile</key>
  <string>pomodoro</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleShortVersionString</key>
  <string>0.1.0</string>
  <key>CFBundleVersion</key>
  <string>1</string>
  <key>CFBundleSupportedPlatforms</key>
  <array><string>MacOSX</string></array>
  <key>LSMinimumSystemVersion</key>
  <string>11.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>NSSupportsAutomaticGraphicsSwitching</key>
  <true/>
</dict>
</plist>
PLIST

  # Convert 256×256 PNG → .icns using tools that ship with every Mac.
  echo "Converting icon..."
  if command -v sips >/dev/null && command -v iconutil >/dev/null; then
    ICONSET="$(mktemp -d)/pomodoro.iconset"
    mkdir -p "$ICONSET"
    sips -z 16   16   assets/pomodoro.png --out "$ICONSET/icon_16x16.png"      >/dev/null
    sips -z 32   32   assets/pomodoro.png --out "$ICONSET/icon_16x16@2x.png"   >/dev/null
    sips -z 32   32   assets/pomodoro.png --out "$ICONSET/icon_32x32.png"      >/dev/null
    sips -z 64   64   assets/pomodoro.png --out "$ICONSET/icon_32x32@2x.png"   >/dev/null
    sips -z 128  128  assets/pomodoro.png --out "$ICONSET/icon_128x128.png"    >/dev/null
    sips -z 256  256  assets/pomodoro.png --out "$ICONSET/icon_128x128@2x.png" >/dev/null
    sips -z 256  256  assets/pomodoro.png --out "$ICONSET/icon_256x256.png"    >/dev/null
    sips -z 512  512  assets/pomodoro.png --out "$ICONSET/icon_256x256@2x.png" >/dev/null
    sips -z 512  512  assets/pomodoro.png --out "$ICONSET/icon_512x512.png"    >/dev/null
    iconutil -c icns "$ICONSET" --output "$RESOURCES/pomodoro.icns"
    rm -rf "$(dirname "$ICONSET")"
    echo "Icon installed."
  else
    echo "Warning: sips/iconutil not found — icon skipped."
  fi

  LSREG="/System/Library/Frameworks/CoreServices.framework\
/Frameworks/LaunchServices.framework/Support/lsregister"
  if [[ -x "$LSREG" ]]; then
    "$LSREG" -f "$APP" 2>/dev/null || true
  fi

  touch "$APP"

  echo
  echo "Installed to $APP"
  echo "Open Launchpad or press Cmd+Space and search 'Pomodoro'."
  echo "If it doesn't appear yet, run:  killall Dock"
  echo "Or launch directly:  open '$APP'"
}

# ── Linux ─────────────────────────────────────────────────────────────────────

install_linux() {
  BIN_DIR="$HOME/.local/bin"
  ICON_DIR="$HOME/.local/share/icons/hicolor/256x256/apps"
  APPS_DIR="$HOME/.local/share/applications"

  mkdir -p "$BIN_DIR" "$ICON_DIR" "$APPS_DIR"

  echo "Installing binary to $BIN_DIR/pomodoro"
  install -m 0755 target/release/pomodoro "$BIN_DIR/pomodoro"

  echo "Installing icon to $ICON_DIR/pomodoro.png"
  install -m 0644 assets/pomodoro.png "$ICON_DIR/pomodoro.png"

  DESKTOP_SRC="assets/pomodoro.desktop.in"
  DESKTOP_DST="$APPS_DIR/pomodoro.desktop"

  if [[ ! -f "$DESKTOP_SRC" ]]; then
    echo "Error: $DESKTOP_SRC not found." >&2
    exit 1
  fi

  echo "Installing desktop entry to $DESKTOP_DST"
  ESCAPED_BIN="$(printf '%s\n' "$BIN_DIR/pomodoro" | sed 's/[\/&|]/\\&/g')"
  sed "s|@BIN@|$ESCAPED_BIN|g" "$DESKTOP_SRC" > "$DESKTOP_DST"
  chmod 0644 "$DESKTOP_DST"

  if command -v desktop-file-validate >/dev/null; then
    desktop-file-validate "$DESKTOP_DST" || \
      echo "Warning: desktop-file-validate reported issues." >&2
  fi
  if command -v update-desktop-database >/dev/null; then
    update-desktop-database "$APPS_DIR" || true
  fi
  if command -v gtk-update-icon-cache >/dev/null; then
    gtk-update-icon-cache -f "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
  fi

  echo
  echo "Installed. Make sure $BIN_DIR is on your PATH."
  echo "Launch via your app menu (search 'Pomodoro') or run: pomodoro"
}

# ── Main ──────────────────────────────────────────────────────────────────────

echo "Building release binary..."
cargo build --release

OS="$(uname -s)"
case "$OS" in
  Darwin) install_macos ;;
  Linux)  install_linux  ;;
  *)
    echo "Unsupported OS: $OS" >&2
    exit 1
    ;;
esac
