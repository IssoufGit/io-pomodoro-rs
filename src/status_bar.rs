// Menu bar / top bar indicator: "🍅 24 min ▶" plus a dropdown menu.
//
//   macOS — NSStatusItem in the menu bar, dropdown built with muda.
//   Linux — AppIndicator (libayatana-appindicator via tray-icon). The label
//           text shows in Ubuntu's top bar (AppIndicator extension) and on
//           KDE; other hosts show just the icon. Works on X11 and Wayland
//           since it's all D-Bus.
//
// The menu tree and click handling are shared; muda's MenuEvent channel is
// global, so `poll_commands()` can be called from any thread on both OSes.

/// A command produced by clicking an item in the dropdown menu.
pub enum MenuCommand {
    TogglePause,
    Reset,
    StartNewFocus,
    SetFocusDuration(u32),
}

/// Handles to the menu items whose text/checked-state change at runtime.
/// Holding `menu` alive keeps the whole tree (including the duration
/// submenu) alive, since `Menu`/`Submenu` retain their appended children.
struct MenuHandles {
    menu: muda::Menu,
    toggle_pause: muda::MenuItem,
    duration_items: Vec<(u32, muda::CheckMenuItem)>,
}

impl MenuHandles {
    /// Updates the Pause/Start label and the checked focus-duration preset.
    fn refresh(&self, running: bool, focus_min: u32) {
        self.toggle_pause.set_text(if running { "Pause" } else { "Start" });
        for (preset, item) in &self.duration_items {
            item.set_checked(*preset == focus_min);
        }
    }
}

fn build_menu(focus_min: u32) -> MenuHandles {
    use muda::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};

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

    MenuHandles { menu, toggle_pause, duration_items }
}

/// Drains menu clicks since the last call. Cheap no-op when nothing was
/// clicked.
pub fn poll_commands() -> Vec<MenuCommand> {
    let mut commands = Vec::new();
    while let Ok(event) = muda::MenuEvent::receiver().try_recv() {
        let id: &str = event.id.as_ref();
        if id == "toggle_pause" {
            commands.push(MenuCommand::TogglePause);
        } else if id == "reset" {
            commands.push(MenuCommand::Reset);
        } else if id == "start_new_focus" {
            commands.push(MenuCommand::StartNewFocus);
        } else if let Some(min) = id.strip_prefix("duration_").and_then(|s| s.parse::<u32>().ok()) {
            commands.push(MenuCommand::SetFocusDuration(min));
        }
    }
    commands
}

#[cfg(target_os = "macos")]
pub use macos::{refresh_menu, setup, update_text};
#[cfg(target_os = "linux")]
pub use linux::{refresh_menu, setup, update_text};

// ── macOS: NSStatusItem ───────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
mod macos {
    use super::MenuHandles;

    /// Pointer to the NSStatusItem. Accessed only on the main thread.
    static mut STATUS_ITEM: *mut objc::runtime::Object = std::ptr::null_mut();

    /// Main-thread only, like `STATUS_ITEM`.
    static mut MENU: Option<MenuHandles> = None;

    /// Creates the menu bar item and attaches the dropdown menu. Call once on
    /// first frame.
    pub fn setup(focus_min: u32) {
        use muda::ContextMenu;
        use objc::{class, msg_send, sel, sel_impl, runtime::Object};
        use std::ffi::CString;
        unsafe {
            let bar: *mut Object = msg_send![class!(NSStatusBar), systemStatusBar];
            let item: *mut Object = msg_send![bar, statusItemWithLength: -1.0f64];
            let item: *mut Object = msg_send![item, retain];
            STATUS_ITEM = item;
            let btn: *mut Object = msg_send![item, button];
            if !btn.is_null() {
                let s = CString::new("🍅").unwrap();
                let ns: *mut Object = msg_send![class!(NSString), stringWithUTF8String: s.as_ptr()];
                let _: () = msg_send![btn, setTitle: ns];
            }

            let handles = super::build_menu(focus_min);
            let ptr = handles.menu.ns_menu() as *mut Object;
            let _: () = msg_send![STATUS_ITEM, setMenu: ptr];
            MENU = Some(handles);
        }
    }

    /// Updates the menu bar item text. Safe to call from any thread.
    pub fn update_text(text: &str) {
        use objc::{class, msg_send, sel, sel_impl, runtime::Object};
        use std::ffi::CString;
        unsafe {
            if STATUS_ITEM.is_null() { return; }
            let btn: *mut Object = msg_send![STATUS_ITEM, button];
            if btn.is_null() { return; }
            if let Ok(s) = CString::new(text) {
                let ns: *mut Object = msg_send![class!(NSString), stringWithUTF8String: s.as_ptr()];
                let _: () = msg_send![btn, setTitle: ns];
            }
        }
    }

    /// Updates the Pause/Start label and the checked focus-duration preset to
    /// match current app state. Cheap enough to call every frame, same as
    /// `update_text`.
    pub fn refresh_menu(running: bool, focus_min: u32) {
        unsafe {
            let Some(handles) = (*std::ptr::addr_of!(MENU)).as_ref() else { return };
            handles.refresh(running, focus_min);
        }
    }
}

// ── Linux: AppIndicator via tray-icon ─────────────────────────────────────────

#[cfg(target_os = "linux")]
mod linux {
    use std::sync::Mutex;

    /// Latest state requested by the app, applied by the GTK thread.
    struct Pending {
        text: Option<String>,
        menu: Option<(bool, u32)>,
    }

    static PENDING: Mutex<Pending> = Mutex::new(Pending { text: None, menu: None });

    /// Spawns the tray thread. GTK isn't thread-safe and `TrayIcon` isn't
    /// `Send`, so the indicator, its menu and the GTK main loop all live on
    /// that one thread; the rest of the app only talks to it via `PENDING`.
    ///
    /// If the tray can't be created (no libayatana-appindicator installed —
    /// tray-icon panics then — or no D-Bus session), only this thread dies and
    /// the app keeps running without an indicator.
    pub fn setup(focus_min: u32) {
        let spawned = std::thread::Builder::new()
            .name("tray".into())
            .spawn(move || {
                if let Err(err) = gtk::init() {
                    eprintln!("pomodoro: top bar indicator disabled, GTK init failed: {err}");
                    return;
                }
                let handles = super::build_menu(focus_min);
                let icon = crate::icon::make_icon();
                let icon = match tray_icon::Icon::from_rgba(icon.rgba, icon.width, icon.height) {
                    Ok(icon) => icon,
                    Err(err) => {
                        eprintln!("pomodoro: top bar indicator disabled, bad icon: {err}");
                        return;
                    }
                };
                let tray = match tray_icon::TrayIconBuilder::new()
                    .with_menu(Box::new(handles.menu.clone()))
                    .with_icon(icon)
                    .with_tooltip("Pomodoro")
                    // On Linux the title is the text label next to the icon.
                    .with_title("🍅")
                    .build()
                {
                    Ok(tray) => tray,
                    Err(err) => {
                        eprintln!("pomodoro: top bar indicator disabled: {err}");
                        return;
                    }
                };

                // Only touch GTK when something actually changed; the app
                // queues the same state several times a second.
                let mut shown_text = String::new();
                let mut shown_menu = None;
                gtk::glib::timeout_add_local(std::time::Duration::from_millis(300), move || {
                    let (text, menu) = match PENDING.lock() {
                        Ok(mut p) => (p.text.take(), p.menu.take()),
                        Err(_) => (None, None),
                    };
                    if let Some(text) = text.filter(|t| *t != shown_text) {
                        tray.set_title(Some(&text));
                        shown_text = text;
                    }
                    if let Some(state) = menu.filter(|m| Some(*m) != shown_menu) {
                        handles.refresh(state.0, state.1);
                        shown_menu = Some(state);
                    }
                    gtk::glib::ControlFlow::Continue
                });
                gtk::main();
            });
        if let Err(err) = spawned {
            eprintln!("pomodoro: failed to start top bar indicator thread: {err}");
        }
    }

    /// Queues a new label text. Safe to call from any thread.
    pub fn update_text(text: &str) {
        if let Ok(mut p) = PENDING.lock() {
            p.text = Some(text.to_owned());
        }
    }

    /// Queues a Pause/Start label + checked-preset refresh. Safe to call from
    /// any thread.
    pub fn refresh_menu(running: bool, focus_min: u32) {
        if let Ok(mut p) = PENDING.lock() {
            p.menu = Some((running, focus_min));
        }
    }
}
