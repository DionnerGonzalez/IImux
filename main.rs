// llmux — a developer-first CLI for working with multiple LLM providers
// built in Rust because life is too short for slow tools

mod commands;
mod providers;
mod tui;
mod utils;

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;

#[derive(Parser)]
#[command(
    name = "llmux",
    about = "A developer-first toolkit for LLM APIs",
    long_about = "\nllmux lets you chat, benchmark, and compare LLM providers\nfrom a single terminal interface.\n\nSupported providers: OpenAI · Gemini · Anthropic · Ollama",
    version,
    author
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Suppress banners and decorations (useful for piping output)
    #[arg(long, global = true)]
    quiet: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Start an interactive chat session with any LLM provider
    Chat(commands::chat::ChatArgs),

    /// Benchmark response time and token throughput across providers
    Bench(commands::bench::BenchArgs),

    /// Compare responses from multiple models side-by-side
    Compare(commands::compare::CompareArgs),

    /// Manage provider API keys and configuration
    Config(commands::config::ConfigArgs),

    /// Inspect, count, and estimate cost of tokens in a file or string
    Tokens(commands::tokens::TokensArgs),

    /// Show usage stats from your session history
    Stats(commands::stats::StatsArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if !cli.quiet {
        print_banner();
    }

    match cli.command {
        Commands::Chat(args)    => commands::chat::run(args).await?,
        Commands::Bench(args)   => commands::bench::run(args).await?,
        Commands::Compare(args) => commands::compare::run(args).await?,
        Commands::Config(args)  => commands::config::run(args).await?,
        Commands::Tokens(args)  => commands::tokens::run(args).await?,
        Commands::Stats(args)   => commands::stats::run(args).await?,
    }

    Ok(())
}

fn print_banner() {
    println!();
    println!("  {}  {}", "llmux".bold().cyan(), "v0.1.0".dimmed());
    println!("  {}", "─────────────────────────".dimmed());
    println!("  {} {}", "▸".cyan(), "LLM toolkit for developers".white());
    println!();
}
