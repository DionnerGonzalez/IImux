// utils/config.rs — reads and writes ~/.config/llmux/config.toml
//
// The config file stores API keys and default preferences.
// We never print keys to stdout — only use them at call time.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    /// API keys indexed by provider name (lowercase)
    #[serde(default)]
    pub keys: HashMap<String, String>,

    /// Default provider to use when none is specified
    pub default_provider: Option<String>,

    /// Default model override per provider
    #[serde(default)]
    pub models: HashMap<String, String>,
}

impl Config {
    pub fn path() -> Result<PathBuf> {
        let base = dirs::config_dir()
            .context("could not determine config directory")?;
        Ok(base.join("llmux").join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&raw)
            .with_context(|| format!("parsing {}", path.display()))
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let raw = toml::to_string_pretty(self)?;
        fs::write(&path, raw)?;
        Ok(())
    }

    pub fn get_key(&self, provider: &str) -> Option<&str> {
        self.keys.get(provider).map(String::as_str)
    }

    pub fn set_key(&mut self, provider: String, key: String) {
        self.keys.insert(provider, key);
    }

    pub fn get_model(&self, provider: &str) -> Option<&str> {
        self.models.get(provider).map(String::as_str)
    }
}
