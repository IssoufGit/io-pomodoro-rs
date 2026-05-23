// Platform-specific helpers for features eframe/winit don't expose uniformly.
//
// Always-on-top support matrix:
//   macOS   — ViewportCommand::WindowLevel works via NSWindow level.
//   Windows — ViewportCommand::WindowLevel works via SetWindowPos HWND_TOPMOST.
//   Linux X11      — ViewportCommand + wmctrl (sends the required _NET_WM_STATE
//                    ClientMessage that some winit versions omit).
//   Linux Wayland  — Not reliably supported. Standard Wayland protocols have no
//                    always-on-top concept; only compositor-specific extensions
//                    (wlr-layer-shell, etc.) expose it and winit does not use them.
//                    The button is disabled with an explanatory tooltip in this case.

// ── Public API ────────────────────────────────────────────────────────────────

/// True when we should let the OS/compositor draw the window decorations.
/// On Wayland this gives back the compositor right-click menu (including
/// "Always on top" on KDE) and avoids the egui custom title bar.
/// On X11, macOS and Windows we draw our own title bar.
pub fn using_os_decorations() -> bool {
    #[cfg(target_os = "linux")]
    return is_wayland();

    #[cfg(not(target_os = "linux"))]
    false
}

/// Whether always-on-top can be set on this platform/session.
/// Used by the UI to enable or grey out the pin button.
pub fn always_on_top_supported() -> bool {
    #[cfg(target_os = "linux")]
    return !is_wayland();

    #[cfg(not(target_os = "linux"))]
    true
}

/// Apply always-on-top. `egui_ctx` must also send `ViewportCommand::WindowLevel`
/// separately (done in `PomodoroApp::apply_always_on_top`).
/// This supplements that with platform-specific mechanisms where needed.
pub fn set_always_on_top(window_title: &str, enable: bool) {
    #[cfg(target_os = "linux")]
    linux::set_always_on_top(window_title, enable);

    // Silence unused-variable warnings on platforms handled purely by
    // ViewportCommand::WindowLevel (macOS, Windows).
    let _ = (window_title, enable);
}

// ── Wayland detection ─────────────────────────────────────────────────────────

/// True when the current session is a native Wayland session.
/// Checks XDG_SESSION_TYPE; falls back to probing WAYLAND_DISPLAY.
#[cfg(target_os = "linux")]
pub fn is_wayland() -> bool {
    if let Ok(t) = std::env::var("XDG_SESSION_TYPE") {
        return t.eq_ignore_ascii_case("wayland");
    }
    // Fallback: WAYLAND_DISPLAY set and DISPLAY absent/empty means Wayland.
    std::env::var("WAYLAND_DISPLAY").is_ok()
        && std::env::var("DISPLAY").map(|d| d.is_empty()).unwrap_or(true)
}

// ── Linux X11 backend ─────────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
mod linux {
    /// On X11, `wmctrl` sends the `_NET_WM_STATE_ABOVE` ClientMessage to the
    /// root window, which is what window managers actually listen for.
    /// Install with: sudo apt install wmctrl  (or dnf / pacman equivalent).
    /// Silently does nothing if wmctrl is absent or the session is Wayland.
    pub fn set_always_on_top(window_title: &str, enable: bool) {
        if super::is_wayland() {
            return;
        }
        let op = if enable { "add" } else { "remove" };
        let _ = std::process::Command::new("wmctrl")
            .args(["-r", window_title, "-b", &format!("{},above", op)])
            .spawn();
    }
}
