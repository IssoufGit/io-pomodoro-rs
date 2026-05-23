// All disk I/O: loading and saving state, exporting CSV.
// No business logic here — pure serialisation / file handling.

use crate::model::{APP_NAME, Mode, PersistedState, Session};
use chrono::Local;
use std::path::PathBuf;

// ── Paths ─────────────────────────────────────────────────────────────────────

pub fn data_dir() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".local")
            .join("share")
    });
    base.join(APP_NAME)
}

pub fn data_file_path() -> PathBuf {
    data_dir().join("history.json")
}

// ── Load / Save ───────────────────────────────────────────────────────────────

pub fn load_state() -> Option<PersistedState> {
    let bytes = std::fs::read(data_file_path()).ok()?;
    serde_json::from_slice::<PersistedState>(&bytes).ok()
}

/// Atomic write: write to a temp file, then rename into place.
/// Prevents corruption if the process is killed mid-write.
pub fn save_state(state: &PersistedState) -> std::io::Result<()> {
    let dir = data_dir();
    std::fs::create_dir_all(&dir)?;
    let path = data_file_path();
    let tmp = dir.join("history.json.tmp");
    let bytes = serde_json::to_vec_pretty(state)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}

// ── CSV Export ────────────────────────────────────────────────────────────────

pub fn export_csv(sessions: &[Session]) -> std::io::Result<()> {
    let dir = data_dir();
    std::fs::create_dir_all(&dir)?;
    let stamp = Local::now().format("%Y-%m-%d").to_string();
    let path = dir.join(format!("pomodoro-history-{}.csv", stamp));

    let mut out = String::from("completed_at,mode,duration_minutes,completed\n");
    for s in sessions.iter().rev() {
        let mode = match s.mode {
            Mode::Focus => "focus",
            Mode::ShortBreak => "short_break",
            Mode::LongBreak => "long_break",
        };
        out.push_str(&format!(
            "{},{},{},{}\n",
            s.completed_at.to_rfc3339(),
            mode,
            s.duration_minutes,
            if s.completed { "yes" } else { "no" },
        ));
    }
    std::fs::write(&path, out)?;
    Ok(())
}
