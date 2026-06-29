// utils/history.rs — persists chat sessions to ~/.local/share/llmux/

use crate::providers::{Message, ProviderKind};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub provider: ProviderKind,
    pub model: String,
    pub created_at: DateTime<Utc>,
    pub messages: Vec<Message>,
    /// Total tokens consumed across the session
    pub total_tokens: u32,
    /// Total wall-clock time waiting for the API, in ms
    pub total_latency_ms: u128,
}

impl Session {
    pub fn new(provider: ProviderKind, model: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            provider,
            model,
            created_at: Utc::now(),
            messages: Vec::new(),
            total_tokens: 0,
            total_latency_ms: 0,
        }
    }

    pub fn push(&mut self, msg: Message) {
        self.messages.push(msg);
    }

    pub fn record_turn(&mut self, tokens: Option<u32>, latency_ms: u128) {
        if let Some(t) = tokens {
            self.total_tokens += t;
        }
        self.total_latency_ms += latency_ms;
    }

    pub fn turns(&self) -> usize {
        // Each turn = one user message + one assistant reply
        self.messages.iter().filter(|m| matches!(m.role, crate::providers::Role::User)).count()
    }
}

pub fn data_dir() -> Result<PathBuf> {
    let base = dirs::data_local_dir()
        .context("could not determine local data directory")?;
    Ok(base.join("llmux").join("sessions"))
}

pub fn save_session(session: &Session) -> Result<()> {
    let dir = data_dir()?;
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", session.id));
    let json = serde_json::to_string_pretty(session)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn load_all_sessions() -> Result<Vec<Session>> {
    let dir = data_dir()?;
    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut sessions = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            let raw = fs::read_to_string(&path)?;
            if let Ok(s) = serde_json::from_str::<Session>(&raw) {
                sessions.push(s);
            }
        }
    }

    // Newest first
    sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(sessions)
}
