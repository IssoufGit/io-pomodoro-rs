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
    return !is_wayland() && linux::wmctrl_available();

    #[cfg(not(target_os = "linux"))]
    true
}

/// Explains why the pin button is disabled, if it is. `None` means
/// always-on-top is supported (or this isn't Linux, where it's always
/// supported via ViewportCommand::WindowLevel alone).
#[cfg(target_os = "linux")]
pub fn always_on_top_unsupported_reason() -> Option<&'static str> {
    if is_wayland() {
        Some("not supported on Wayland")
    } else if !linux::wmctrl_available() {
        Some("install \"wmctrl\" to enable (e.g. sudo apt install wmctrl)")
    } else {
        None
    }
}

#[cfg(not(target_os = "linux"))]
pub fn always_on_top_unsupported_reason() -> Option<&'static str> {
    None
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
    /// True when the `wmctrl` binary is found on `PATH`. Cached after the
    /// first check since PATH doesn't change during the app's lifetime.
    pub fn wmctrl_available() -> bool {
        static AVAILABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *AVAILABLE.get_or_init(|| {
            std::env::var_os("PATH")
                .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join("wmctrl").is_file()))
                .unwrap_or(false)
        })
    }

    /// On X11, `wmctrl` sends the `_NET_WM_STATE_ABOVE` ClientMessage to the
    /// root window, which is what window managers actually listen for.
    /// Install with: sudo apt install wmctrl  (or dnf / pacman equivalent).
    /// Does nothing if the session is Wayland; the UI gates on
    /// `wmctrl_available()` so this normally only runs when the binary exists.
    pub fn set_always_on_top(window_title: &str, enable: bool) {
        if super::is_wayland() {
            return;
        }
        let op = if enable { "add" } else { "remove" };
        if let Err(err) = std::process::Command::new("wmctrl")
            .args(["-r", window_title, "-b", &format!("{op},above")])
            .spawn()
        {
            eprintln!("pomodoro: failed to run wmctrl: {err}");
        }
    }
}

// ── macOS: window management & status bar ─────────────────────────────────────

/// Returns the app's main content window, falling back to the first entry
/// in `[NSApplication windows]` when `mainWindow` is nil (e.g. very early
/// during startup, before the window has become key/main — see
/// `window_is_minimized` for the same class of issue).
#[cfg(target_os = "macos")]
fn app_window() -> *mut objc::runtime::Object {
    use objc::{class, msg_send, sel, sel_impl, runtime::Object};
    unsafe {
        let app: *mut Object = msg_send![class!(NSApplication), sharedApplication];
        let window: *mut Object = msg_send![app, mainWindow];
        if !window.is_null() {
            return window;
        }
        let windows: *mut Object = msg_send![app, windows];
        let count: usize = msg_send![windows, count];
        if count > 0 {
            return msg_send![windows, objectAtIndex: 0usize];
        }
        std::ptr::null_mut()
    }
}

/// Minimize the main window.
/// Deferred via performSelector:withObject:afterDelay: so it runs after the
/// current Metal frame is fully presented — direct miniaturize: inside a frame
/// hangs the app on macOS.
pub fn minimize_window(ctx: &eframe::egui::Context) {
    #[cfg(target_os = "macos")]
    {
        use objc::{msg_send, sel, sel_impl, runtime::Object};
        unsafe {
            let win = app_window();
            if !win.is_null() {
                let nil: *mut Object = std::ptr::null_mut();
                let _: () = msg_send![
                    win,
                    performSelector: sel!(miniaturize:)
                    withObject: nil
                    afterDelay: 0.0f64
                ];
            }
        }
        let _ = ctx;
    }
    #[cfg(not(target_os = "macos"))]
    ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Minimized(true));
}

/// Round (or square back up) the window corners. Used by tiny mode's strip.
///
/// This clips the content view's layer in the compositor rather than relying
/// on the Metal surface rendering alpha (which never worked, see git history),
/// so egui keeps drawing an opaque background.
pub fn set_rounded_corners(enable: bool) {
    #[cfg(target_os = "macos")]
    {
        use objc::{class, msg_send, sel, sel_impl, runtime::{Object, NO, YES}};
        unsafe {
            let win = app_window();
            if win.is_null() {
                return;
            }
            let _: () = msg_send![win, setOpaque: NO];
            let clear: *mut Object = msg_send![class!(NSColor), clearColor];
            let _: () = msg_send![win, setBackgroundColor: clear];
            let view: *mut Object = msg_send![win, contentView];
            if !view.is_null() {
                let _: () = msg_send![view, setWantsLayer: YES];
                let layer: *mut Object = msg_send![view, layer];
                if !layer.is_null() {
                    let radius: f64 = if enable { 12.0 } else { 0.0 };
                    let _: () = msg_send![layer, setCornerRadius: radius];
                    let _: () = msg_send![layer, setMasksToBounds: if enable { YES } else { NO }];
                }
            }
            let _: () = msg_send![win, invalidateShadow];
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = enable;
}

/// Returns true when any app window is currently miniaturised (in the Dock).
///
/// [NSApplication mainWindow] returns nil while a window is miniaturised —
/// miniaturised windows are not the main/key window. We must iterate the
/// full [NSApplication windows] array which includes miniaturised windows.
#[cfg(target_os = "macos")]
pub fn window_is_minimized() -> bool {
    use objc::{class, msg_send, sel, sel_impl, runtime::Object};
    unsafe {
        let app: *mut Object = msg_send![class!(NSApplication), sharedApplication];
        let windows: *mut Object = msg_send![app, windows];
        let count: usize = msg_send![windows, count];
        for i in 0..count {
            let win: *mut Object = msg_send![windows, objectAtIndex: i];
            if !win.is_null() {
                let mini: bool = msg_send![win, isMiniaturized];
                if mini { return true; }
            }
        }
        false
    }
}


/// Pointer to the NSStatusItem. Accessed only on the main thread.
#[cfg(target_os = "macos")]
static mut MACOS_STATUS_ITEM: *mut objc::runtime::Object = std::ptr::null_mut();

/// Creates a persistent item in the macOS menu bar. Call once on first frame.
#[cfg(target_os = "macos")]
pub fn setup_macos_status_bar() {
    use objc::{class, msg_send, sel, sel_impl, runtime::Object};
    use std::ffi::CString;
    unsafe {
        let bar: *mut Object = msg_send![class!(NSStatusBar), systemStatusBar];
        let item: *mut Object = msg_send![bar, statusItemWithLength: -1.0f64];
        let item: *mut Object = msg_send![item, retain];
        MACOS_STATUS_ITEM = item;
        let btn: *mut Object = msg_send![item, button];
        if !btn.is_null() {
            let s = CString::new("🍅").unwrap();
            let ns: *mut Object = msg_send![class!(NSString), stringWithUTF8String: s.as_ptr()];
            let _: () = msg_send![btn, setTitle: ns];
        }
    }
}

/// Updates the menu bar item text. Safe to call from any thread.
#[cfg(target_os = "macos")]
pub fn update_macos_status_bar(text: &str) {
    use objc::{class, msg_send, sel, sel_impl, runtime::Object};
    use std::ffi::CString;
    unsafe {
        if MACOS_STATUS_ITEM.is_null() { return; }
        let btn: *mut Object = msg_send![MACOS_STATUS_ITEM, button];
        if btn.is_null() { return; }
        if let Ok(s) = CString::new(text) {
            let ns: *mut Object = msg_send![class!(NSString), stringWithUTF8String: s.as_ptr()];
            let _: () = msg_send![btn, setTitle: ns];
        }
    }
}

// ── macOS: status bar dropdown menu (via muda) ─────────────────────────────

/// A command produced by clicking an item in the status bar dropdown menu.
#[cfg(target_os = "macos")]
pub enum MacMenuCommand {
    TogglePause,
    Reset,
    StartNewFocus,
    SetFocusDuration(u32),
}

/// Handles to the menu items whose text/checked-state change at runtime.
/// Holding `menu` alive keeps the whole tree (including the duration
/// submenu) alive, since `Menu`/`Submenu` retain their appended children.
#[cfg(target_os = "macos")]
struct MacMenuHandles {
    #[allow(dead_code)]
    menu: muda::Menu,
    toggle_pause: muda::MenuItem,
    duration_items: Vec<(u32, muda::CheckMenuItem)>,
}

/// Main-thread only, like `MACOS_STATUS_ITEM`.
#[cfg(target_os = "macos")]
static mut MACOS_MENU: Option<MacMenuHandles> = None;

/// Builds the status bar dropdown menu and attaches it to the status item
/// created by `setup_macos_status_bar()`. Call once on first frame, after
/// that function.
#[cfg(target_os = "macos")]
pub fn setup_macos_status_menu(focus_min: u32) {
    use muda::{CheckMenuItem, ContextMenu, Menu, MenuItem, PredefinedMenuItem, Submenu};
    use objc::{msg_send, sel, sel_impl, runtime::Object};

    let menu = Menu::new();
    let toggle_pause = MenuItem::with_id("toggle_pause", "Start", true, None);
    let reset = MenuItem::with_id("reset", "Reset", true, None);
    let start_new_focus = MenuItem::with_id("start_new_focus", "Start New Focus", true, None);

    let duration_submenu = Submenu::new("Focus Duration", true);
    let mut duration_items = Vec::new();
    for preset in crate::model::FOCUS_PRESETS {
        let item = CheckMenuItem::with_id(
            format!("duration_{preset}"),
            format!("{preset} min"),
            true,
            preset == focus_min,
            None,
        );
        let _ = duration_submenu.append(&item);
        duration_items.push((preset, item));
    }

    let _ = menu.append(&toggle_pause);
    let _ = menu.append(&reset);
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&start_new_focus);
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&duration_submenu);

    unsafe {
        if !MACOS_STATUS_ITEM.is_null() {
            let ptr = menu.ns_menu() as *mut Object;
            let _: () = msg_send![MACOS_STATUS_ITEM, setMenu: ptr];
        }
        MACOS_MENU = Some(MacMenuHandles { menu, toggle_pause, duration_items });
    }
}

/// Updates the Pause/Start label and the checked focus-duration preset to
/// match current app state. Main-thread only; cheap enough to call every
/// frame, same as `update_macos_status_bar`.
#[cfg(target_os = "macos")]
pub fn refresh_macos_menu(running: bool, focus_min: u32) {
    unsafe {
        let Some(handles) = MACOS_MENU.as_ref() else { return };
        handles.toggle_pause.set_text(if running { "Pause" } else { "Start" });
        for (preset, item) in &handles.duration_items {
            item.set_checked(*preset == focus_min);
        }
    }
}

/// Drains status bar menu clicks since the last call. Main-thread only;
/// safe/cheap no-op when nothing was clicked.
#[cfg(target_os = "macos")]
pub fn poll_macos_menu_commands() -> Vec<MacMenuCommand> {
    let mut commands = Vec::new();
    while let Ok(event) = muda::MenuEvent::receiver().try_recv() {
        let id: &str = event.id.as_ref();
        if id == "toggle_pause" {
            commands.push(MacMenuCommand::TogglePause);
        } else if id == "reset" {
            commands.push(MacMenuCommand::Reset);
        } else if id == "start_new_focus" {
            commands.push(MacMenuCommand::StartNewFocus);
        } else if let Some(min) = id.strip_prefix("duration_").and_then(|s| s.parse::<u32>().ok()) {
            commands.push(MacMenuCommand::SetFocusDuration(min));
        }
    }
    commands
}
