// commands/bench.rs — benchmark latency and throughput across providers
//
// Sends the same prompt N times and reports P50/P95 latency + tokens/sec.
//
// Usage:
//   llmux bench "Explain Rust's ownership model in one sentence"
//   llmux bench "Hello" --providers openai,gemini --runs 5

use crate::providers::{self, Message, ProviderKind, Role};
use crate::utils::config::Config;
use anyhow::Result;
use clap::Args;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

#[derive(Args, Debug)]
pub struct BenchArgs {
    /// The prompt to benchmark
    pub prompt: String,

    /// Comma-separated list of providers (default: all configured)
    #[arg(short, long, default_value = "gemini")]
    pub providers: String,

    /// Number of runs per provider
    #[arg(short, long, default_value = "3")]
    pub runs: usize,

    /// Model override (same model used for all providers if set)
    #[arg(short, long)]
    pub model: Option<String>,
}

struct RunResult {
    latency_ms: u128,
    tokens: Option<u32>,
}

pub async fn run(args: BenchArgs) -> Result<()> {
    let config = Config::load()?;

    let provider_names: Vec<&str> = args.providers.split(',').collect();

    println!();
    println!("  {} {}", "benchmark".bold(), format!("· {} runs per provider", args.runs).dimmed());
    println!("  {} {}", "prompt:".dimmed(), truncate(&args.prompt, 60).white());
    println!();

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

        let provider = providers::build(kind, api_key, args.model.clone());

        let messages = vec![Message {
            role: Role::User,
            content: args.prompt.clone(),
        }];

        let bar = ProgressBar::new(args.runs as u64);
        bar.set_style(
            ProgressStyle::default_bar()
                .template(&format!(
                    "  {{spinner:.cyan}} {} {{bar:20.cyan/blue}} {{pos}}/{{len}}",
                    format!("{kind}").color(kind.color_tag())
                ))
                .unwrap()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
        );
        bar.enable_steady_tick(Duration::from_millis(80));

        let mut results: Vec<RunResult> = Vec::new();

        for _ in 0..args.runs {
            match provider.complete(&messages).await {
                Ok(resp) => {
                    results.push(RunResult {
                        latency_ms: resp.latency_ms,
                        tokens: resp.tokens_used,
                    });
                }
                Err(e) => {
                    bar.println(format!("  {} {e}", "error:".red()));
                }
            }
            bar.inc(1);
        }

        bar.finish_and_clear();

        if results.is_empty() {
            println!("  {} {} all runs failed\n", "✗".red(), format!("{kind}").color(kind.color_tag()));
            continue;
        }

        // Compute stats
        let mut latencies: Vec<u128> = results.iter().map(|r| r.latency_ms).collect();
        latencies.sort_unstable();

        let p50 = percentile(&latencies, 50);
        let p95 = percentile(&latencies, 95);
        let avg = latencies.iter().sum::<u128>() / latencies.len() as u128;

        let avg_tokens: Option<f64> = {
            let ts: Vec<u32> = results.iter().filter_map(|r| r.tokens).collect();
            if ts.is_empty() {
                None
            } else {
                Some(ts.iter().sum::<u32>() as f64 / ts.len() as f64)
            }
        };

        let tokens_per_sec = avg_tokens.map(|t| {
            let secs = avg as f64 / 1000.0;
            if secs > 0.0 { t / secs } else { 0.0 }
        });

        println!(
            "  {} {}",
            "✓".green(),
            format!("{} / {}", kind, provider.model()).color(kind.color_tag()).bold(),
        );
        println!("  {}   avg {}ms  p50 {}ms  p95 {}ms",
            "  latency:".dimmed(),
            avg.to_string().white(),
            p50.to_string().white(),
            p95.to_string().white(),
        );
        if let Some(tps) = tokens_per_sec {
            println!("  {} {:.0} tokens/sec  (~{:.0} tokens/call)",
                "  throughput:".dimmed(),
                tps.to_string().white().parse::<f64>().unwrap_or(tps),
                avg_tokens.unwrap_or(0.0),
            );
        }
        println!();
    }

    Ok(())
}

fn percentile(sorted: &[u128], p: usize) -> u128 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() - 1) * p) / 100;
    sorted[idx]
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}
