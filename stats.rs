// commands/stats.rs — session history and usage statistics
//
// Usage:
//   llmux stats
//   llmux stats --last 10

use crate::providers::ProviderKind;
use crate::utils::history;
use anyhow::Result;
use clap::Args;
use colored::Colorize;
use std::collections::HashMap;

#[derive(Args, Debug)]
pub struct StatsArgs {
    /// How many recent sessions to show (default: 20)
    #[arg(short, long, default_value = "20")]
    pub last: usize,
}

pub async fn run(args: StatsArgs) -> Result<()> {
    let sessions = history::load_all_sessions()?;

    if sessions.is_empty() {
        println!();
        println!("  {} no sessions recorded yet", "→".dimmed());
        println!("  start one with {}", "llmux chat".cyan());
        println!();
        return Ok(());
    }

    // Aggregate stats
    let total_sessions = sessions.len();
    let total_turns: usize    = sessions.iter().map(|s| s.turns()).sum();
    let total_tokens: u32     = sessions.iter().map(|s| s.total_tokens).sum();
    let total_latency: u128   = sessions.iter().map(|s| s.total_latency_ms).sum();

    let mut by_provider: HashMap<ProviderKind, (usize, u32)> = HashMap::new();
    for s in &sessions {
        let entry = by_provider.entry(s.provider).or_insert((0, 0));
        entry.0 += s.turns();
        entry.1 += s.total_tokens;
    }

    println!();
    println!("  {}", "session stats".bold());
    println!("  {}  {}", "sessions:".dimmed(), total_sessions.to_string().cyan());
    println!("  {}     {}", "turns:".dimmed(), total_turns.to_string().white());
    println!("  {}    {}", "tokens:".dimmed(), total_tokens.to_string().white());
    if total_turns > 0 {
        let avg_latency = total_latency / total_turns as u128;
        println!("  {} {}ms", "avg latency:".dimmed(), avg_latency.to_string().white());
    }
    println!();

    println!("  {}", "by provider".bold());
    let mut provider_list: Vec<_> = by_provider.iter().collect();
    provider_list.sort_by_key(|(_, (turns, _))| std::cmp::Reverse(*turns));
    for (kind, (turns, tokens)) in provider_list {
        println!(
            "  {}  {} turns  {} tokens",
            format!("{kind}:").color(kind.color_tag()),
            turns.to_string().white(),
            tokens.to_string().dimmed(),
        );
    }
    println!();

    // Recent sessions table
    println!("  {}", format!("last {} sessions", args.last.min(total_sessions)).bold());
    for s in sessions.iter().take(args.last) {
        let date = s.created_at.format("%m/%d %H:%M");
        println!(
            "  {} {}  {}  {} turns  {} tokens",
            s.provider.to_string().color(s.provider.color_tag()),
            s.model.dimmed(),
            date.to_string().dimmed(),
            s.turns().to_string().white(),
            s.total_tokens.to_string().dimmed(),
        );
    }
    println!();

    Ok(())
}

// Need this for sorting
use std::cmp::Reverse;
