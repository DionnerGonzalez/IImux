// commands/compare.rs — side-by-side model comparison
//
// Sends the same prompt to multiple providers in parallel and
// shows their responses one after another for easy comparison.
//
// Usage:
//   llmux compare "What is the best way to handle errors in Rust?" --providers openai,anthropic,gemini

use crate::providers::{self, Message, ProviderKind, Role};
use crate::utils::config::Config;
use anyhow::Result;
use clap::Args;
use colored::Colorize;
use tokio::task::JoinSet;

#[derive(Args, Debug)]
pub struct CompareArgs {
    /// The prompt to compare across providers
    pub prompt: String,

    /// Comma-separated providers to compare
    #[arg(short, long, default_value = "gemini,ollama")]
    pub providers: String,

    /// System prompt applied to all providers
    #[arg(short, long)]
    pub system: Option<String>,
}

pub async fn run(args: CompareArgs) -> Result<()> {
    let config = Config::load()?;
    let provider_names: Vec<String> = args
        .providers
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    println!();
    println!("  {} {}", "compare".bold(), format!("· {} providers", provider_names.len()).dimmed());
    println!("  {} {}", "prompt:".dimmed(), args.prompt.white());
    println!();

    // Build messages (shared for all providers)
    let mut messages = Vec::new();
    if let Some(sys) = &args.system {
        messages.push(Message {
            role: Role::System,
            content: sys.clone(),
        });
    }
    messages.push(Message {
        role: Role::User,
        content: args.prompt.clone(),
    });

    // Spawn all providers in parallel
    let mut tasks: JoinSet<(ProviderKind, String, Result<crate::providers::CompletionResponse>)> =
        JoinSet::new();

    for name in &provider_names {
        let kind = match ProviderKind::from_str(name) {
            Some(k) => k,
            None => {
                eprintln!("  {} unknown provider '{}', skipping", "warn:".yellow(), name);
                continue;
            }
        };

        let api_key = config
            .get_key(&name.to_lowercase())
            .map(String::from);

        let provider = providers::build(kind, api_key, None);
        let msgs = messages.clone();
        let model = provider.model().to_string();

        tasks.spawn(async move {
            let result = provider.complete(&msgs).await;
            (kind, model, result)
        });
    }

    // Collect and display results as they arrive
    let mut results = Vec::new();
    while let Some(join_result) = tasks.join_next().await {
        match join_result {
            Ok(r) => results.push(r),
            Err(e) => eprintln!("  {} task failed: {e}", "error:".red()),
        }
    }

    // Sort by latency so the fastest appears first
    results.sort_by_key(|(_, _, r)| {
        r.as_ref().map(|c| c.latency_ms).unwrap_or(u128::MAX)
    });

    for (kind, model, result) in results {
        let header = format!("{kind} / {model}").color(kind.color_tag()).bold();
        println!("  ┌── {header}");

        match result {
            Ok(resp) => {
                for line in resp.content.lines() {
                    println!("  │  {}", line.white());
                }
                println!(
                    "  └── {} {}ms{}",
                    "↳".dimmed(),
                    resp.latency_ms.to_string().green(),
                    resp.tokens_used
                        .map(|t| format!(" · {t} tokens").dimmed().to_string())
                        .unwrap_or_default(),
                );
            }
            Err(e) => {
                println!("  │  {} {e}", "error:".red());
                println!("  └──");
            }
        }

        println!();
    }

    Ok(())
}
