# Pomodoro

[![macOS](https://github.com/IssoufGit/io-pomodoro-rs/actions/workflows/ci-macos.yml/badge.svg)](https://github.com/IssoufGit/io-pomodoro-rs/actions/workflows/ci-macos.yml)
[![Linux](https://github.com/IssoufGit/io-pomodoro-rs/actions/workflows/ci-linux.yml/badge.svg)](https://github.com/IssoufGit/io-pomodoro-rs/actions/workflows/ci-linux.yml)

A local-only pomodoro timer with persistent history. No network, no telemetry.

## Build

```bash
cd io-pomodoro-rs
cargo build --release
```

The binary will be at `target/release/pomodoro` (~8MB).

## Run

```bash
./target/release/pomodoro
```

Or copy it somewhere on your PATH and double-click in your file manager:

```bash
cp target/release/pomodoro ~/.local/bin/
```

## macOS

Only prerequisite is the Xcode Command Line Tools (`xcode-select --install`) —
no extra system libraries needed, unlike Linux.

```bash
cargo build --release
./target/release/pomodoro
```

For a proper app bundle you can launch from Launchpad/Spotlight instead of the
raw binary, run `./install.sh` — it builds the release binary and installs a
`Pomodoro.app` bundle to `~/Applications`.

The menu bar shows a 🍅 status item while the app is running — this is a
macOS-only feature (there's no equivalent tray icon on Linux/X11 yet).

## Why the icon may not show up at first

The icon embedded in the binary works on **X11** (title bar, Alt-Tab, dock — all
of it). On **Wayland** (default on modern GNOME), `with_icon` is a no-op — the
compositor refuses to use it. Wayland matches running apps to icons through a
`.desktop` file via the app's `app_id`, not through the binary itself.

You can check which session you're on:

```bash
echo $XDG_SESSION_TYPE   # "wayland" or "x11"
```

The included `install.sh` handles both cases. It builds the release binary
and installs:

- `~/.local/bin/pomodoro`                            — the executable
- `~/.local/share/icons/hicolor/256x256/apps/pomodoro.png`  — the icon
- `~/.local/share/applications/pomodoro.desktop`     — the entry that ties them

Run it:

```bash
./install.sh
```

After install, search "Pomodoro" in your application launcher. On Wayland, you
may need to log out and back in once for the compositor to pick up the new
app_id — sessions cache app identities aggressively.

## "Double-click to open" on Linux

Once installed via `install.sh`, just search "Pomodoro" in your launcher and pin
it to the dock. If you'd rather wire it up manually, the bits are:

```ini
# ~/.local/share/applications/pomodoro.desktop
[Desktop Entry]
Type=Application
Name=Pomodoro
Comment=Local focus timer
Exec=pomodoro
Icon=pomodoro
Terminal=false
Categories=Utility;
StartupWMClass=pomodoro
```

The `StartupWMClass=pomodoro` line is critical — it must match the `app_id`
the binary registers (`pomodoro`) so the compositor associates the running
window with this `.desktop` entry. Without it, the dock shows a generic icon.

## Data location

History saves to a per-OS app data directory, resolved via the `dirs` crate:

- **Linux** — `$XDG_DATA_HOME/pomodoro/history.json` (typically
  `~/.local/share/pomodoro/history.json`)
- **macOS** — `~/Library/Application Support/pomodoro/history.json`

Plain JSON — back it up however you want.

The Export CSV button writes `pomodoro-history-YYYY-MM-DD.csv` to the same
directory.

## Keyboard shortcuts

- **Space** — start / pause
- **R** — reset the current timer

## Dependencies

Pure-Rust GUI via `egui`/`eframe`. On Linux you need standard X11 or Wayland
libraries — already present on any desktop. Headless servers won't work (no
display). On macOS the Xcode Command Line Tools are the only requirement — no
extra system libs.

If `cargo build` complains about missing system libs, install:

```bash
# Debian/Ubuntu
sudo apt install libxkbcommon-dev libwayland-dev libxcb1-dev

# Fedora
sudo dnf install libxkbcommon-devel wayland-devel libxcb-devel
```

**Always-on-top on Linux X11** additionally requires the `wmctrl` binary on
`PATH` (winit alone doesn't reliably set the required window-manager hint):

```bash
sudo apt install wmctrl   # Debian/Ubuntu
sudo dnf install wmctrl   # Fedora
```

If `wmctrl` isn't installed, the pin button is simply disabled with a tooltip
explaining how to enable it — nothing breaks. Not required on Wayland
(always-on-top is unsupported there by design, same button/tooltip pattern) or
on macOS (handled natively via `NSWindow` levels).
