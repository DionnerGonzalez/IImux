// commands/tokens.rs — inspect, count, and estimate cost for text
//
// Usage:
//   llmux tokens "some text to analyze"
//   llmux tokens --file prompt.txt --provider anthropic

use crate::providers::ProviderKind;
use crate::utils::tokens::{estimate_tokens, format_cost, TokenPricing};
use anyhow::Result;
use clap::Args;
use colored::Colorize;
use std::fs;

#[derive(Args, Debug)]
pub struct TokensArgs {
    /// Text to analyze (pass as argument or use --file)
    pub text: Option<String>,

    /// Read text from file
    #[arg(short, long)]
    pub file: Option<String>,

    /// Provider to use for cost estimation (default: all)
    #[arg(short, long)]
    pub provider: Option<String>,
}

pub async fn run(args: TokensArgs) -> Result<()> {
    let text = if let Some(f) = &args.file {
        fs::read_to_string(f)?
    } else if let Some(t) = &args.text {
        t.clone()
    } else {
        // Read from stdin if no input provided
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        buf
    };

    if text.is_empty() {
        eprintln!("  {} no input provided", "error:".red());
        return Ok(());
    }

    let token_count = estimate_tokens(&text);
    let char_count  = text.chars().count();
    let word_count  = text.split_whitespace().count();
    let line_count  = text.lines().count();

    println!();
    println!("  {}", "token analysis".bold());
    println!("  {}  ~{}", "tokens:".dimmed(), token_count.to_string().cyan().bold());
    println!("  {}   {}", "chars:".dimmed(), char_count.to_string().white());
    println!("  {}   {}", "words:".dimmed(), word_count.to_string().white());
    println!("  {}   {}", "lines:".dimmed(), line_count.to_string().white());
    println!();

    // Cost estimation
    println!("  {}", "estimated cost (as output tokens)".bold());
    let providers = if let Some(p) = &args.provider {
        vec![p.as_str().to_string()]
    } else {
        vec!["openai".to_string(), "anthropic".to_string(), "gemini".to_string()]
    };

    for p in providers {
        let kind = ProviderKind::from_str(&p);
        if let Some(k) = kind {
            if let Some(pricing) = TokenPricing::for_provider(k, k.default_model()) {
                let cost = pricing.estimate_cost(token_count as u32, token_count as u32);
                println!(
                    "  {}  {}",
                    format!("{k}:").dimmed(),
                    format_cost(cost).green(),
                );
            } else {
                println!("  {}  {}", format!("{k}:").dimmed(), "local / free".magenta());
            }
        }
    }

    println!();
    println!("  {}", "note: counts are approximate (4 chars ≈ 1 token heuristic)".dimmed());
    println!();

    Ok(())
}
