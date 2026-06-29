// commands/config.rs — manage API keys and preferences
//
// Usage:
//   llmux config set openai sk-...
//   llmux config set gemini AIza...
//   llmux config list
//   llmux config path

use crate::utils::config::Config;
use anyhow::Result;
use clap::{Args, Subcommand};
use colored::Colorize;

#[derive(Args, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Set an API key for a provider
    Set {
        /// Provider name (openai, anthropic, gemini)
        provider: String,
        /// Your API key
        key: String,
    },
    /// Remove a key
    Unset {
        provider: String,
    },
    /// Set the default provider
    Default {
        provider: String,
    },
    /// List configured keys (masked)
    List,
    /// Print config file path
    Path,
}

pub async fn run(args: ConfigArgs) -> Result<()> {
    let mut config = Config::load()?;

    match args.action {
        ConfigAction::Set { provider, key } => {
            let provider = provider.to_lowercase();
            config.set_key(provider.clone(), key.clone());
            config.save()?;
            println!(
                "  {} key for {} saved  {}",
                "✓".green(),
                provider.bold(),
                format!("[{}…]", &key[..key.len().min(8)]).dimmed(),
            );
        }

        ConfigAction::Unset { provider } => {
            let provider = provider.to_lowercase();
            config.keys.remove(&provider);
            config.save()?;
            println!("  {} removed key for {}", "✓".green(), provider.bold());
        }

        ConfigAction::Default { provider } => {
            config.default_provider = Some(provider.clone());
            config.save()?;
            println!("  {} default provider set to {}", "✓".green(), provider.bold());
        }

        ConfigAction::List => {
            println!();
            if config.keys.is_empty() {
                println!("  {} no API keys configured", "→".dimmed());
                println!("  run {} to add one", "llmux config set <provider> <key>".cyan());
            } else {
                println!("  {}", "configured keys".bold());
                for (provider, key) in &config.keys {
                    let masked = mask_key(key);
                    println!("  {}  {} {}", "·".dimmed(), provider.cyan(), masked.dimmed());
                }
            }
            if let Some(default) = &config.default_provider {
                println!();
                println!("  {} default provider: {}", "→".dimmed(), default.cyan());
            }
            println!();
        }

        ConfigAction::Path => {
            let path = Config::path()?;
            println!("  {}", path.display().to_string().cyan());
        }
    }

    Ok(())
}

fn mask_key(key: &str) -> String {
    if key.len() <= 8 {
        return "••••••••".to_string();
    }
    let visible = &key[..4];
    let dots = "•".repeat(key.len().min(20) - 4);
    format!("{visible}{dots}")
}
