use anyhow::{Context, Result};
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: u64,
    pub title: String,
    pub done: bool,
    /// Set when you press `p` to pin to the top (bold in the UI).
    #[serde(default)]
    pub prioritized: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppState {
    pub date: String,
    pub tasks: Vec<Task>,
}

/// Interactive `rusty` startup: [`NeedsRollover`] when the saved calendar day is behind today.
pub enum SessionStart {
    Fresh(AppState),
    Today(AppState),
    NeedsRollover(AppState),
}

impl AppState {
    pub fn empty_today() -> Self {
        Self {
            date: today_string(),
            tasks: Vec::new(),
        }
    }

    pub fn read_from_disk(path: &PathBuf) -> Result<Self> {
        let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        serde_json::from_str(&raw).with_context(|| "parse state file")
    }

    pub fn read_session_start(path: &PathBuf) -> Result<SessionStart> {
        let today = today_string();

        if !path.exists() {
            let state = Self::empty_today();
            state.save(path)?;
            return Ok(SessionStart::Fresh(state));
        }

        let mut state = Self::read_from_disk(path)?;
        if state.date != today {
            return Ok(SessionStart::NeedsRollover(state));
        }

        state.renumber_ids();
        state.save(path)?;
        Ok(SessionStart::Today(state))
    }

    /// `rusty add` / `rusty delete`: if the file is for a past day, start a fresh today without the recap UI.
    pub fn load_for_cli(path: &PathBuf) -> Result<Self> {
        let today = today_string();

        if !path.exists() {
            let state = Self::empty_today();
            state.save(path)?;
            return Ok(state);
        }

        let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        let mut state: AppState =
            serde_json::from_str(&raw).with_context(|| "parse state file")?;

        if state.date != today {
            state = Self::empty_today();
        }

        state.renumber_ids();
        state.save(path)?;
        Ok(state)
    }

    /// Assigns ids1..=n in current list order (matches row numbers in the UI / CLI).
    pub fn renumber_ids(&mut self) {
        for (i, t) in self.tasks.iter_mut().enumerate() {
            t.id = (i + 1) as u64;
        }
    }

    pub fn save(&self, path: &PathBuf) -> Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
        }
        let raw = serde_json::to_string_pretty(self)?;
        fs::write(path, raw).with_context(|| format!("write {}", path.display()))?;
        Ok(())
    }

    /// Skip disk when `mimic` is true (`--ratatui` preview).
    pub fn persist_to_disk(&self, path: &PathBuf, mimic: bool) -> Result<()> {
        if mimic {
            Ok(())
        } else {
            self.save(path)
        }
    }

    /// Load state for UI preview only: no rollover, no rewrites, file left as-is on disk.
    pub fn read_mimic(path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::empty_today());
        }
        let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        let state: AppState = serde_json::from_str(&raw).with_context(|| "parse state file")?;
        Ok(state)
    }

    pub fn next_id(&self) -> u64 {
        self.tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1
    }
}

pub fn today_string() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}
