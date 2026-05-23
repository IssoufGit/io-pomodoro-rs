// Data types and constants shared across the whole crate.

use chrono::{DateTime, Utc};
use eframe::egui;
use serde::{Deserialize, Serialize};

pub const APP_NAME: &str = "pomodoro";
pub const FULL_SIZE: [f32; 2] = [380.0, 620.0];
pub const TINY_SIZE: [f32; 2] = [280.0, 150.0];
pub const FOCUS_PRESETS: [u32; 5] = [15, 25, 45, 60, 90];

// ── Mode ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Mode {
    Focus,
    ShortBreak,
    LongBreak,
}

impl Mode {
    pub fn label(&self) -> &'static str {
        match self {
            Mode::Focus => "Focus",
            Mode::ShortBreak => "Short break",
            Mode::LongBreak => "Long break",
        }
    }

    pub fn short_label(&self) -> &'static str {
        match self {
            Mode::Focus => "Focus",
            Mode::ShortBreak => "Break",
            Mode::LongBreak => "Long break",
        }
    }

    pub fn color(&self) -> egui::Color32 {
        match self {
            Mode::Focus => egui::Color32::from_rgb(196, 69, 54),
            Mode::ShortBreak => egui::Color32::from_rgb(74, 124, 89),
            Mode::LongBreak => egui::Color32::from_rgb(107, 91, 149),
        }
    }
}

// ── Session ───────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Session {
    pub id: String,
    pub mode: Mode,
    pub duration_minutes: f64,
    pub completed_at: DateTime<Utc>,
    /// true = ran out naturally, false = manually saved partial
    pub completed: bool,
}

// ── Settings ──────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Settings {
    pub focus_min: u32,
    pub short_min: u32,
    pub long_min: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self { focus_min: 25, short_min: 5, long_min: 15 }
    }
}

// ── PersistedState ────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Default)]
pub struct PersistedState {
    pub sessions: Vec<Session>,
    pub settings: Settings,
}
