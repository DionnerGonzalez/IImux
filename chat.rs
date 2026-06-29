// commands/chat.rs — interactive multi-turn chat session
//
// Usage:
//   llmux chat --provider gemini
//   llmux chat --provider openai --model gpt-4o-mini --system "You are a Rust expert"
//   llmux chat --provider ollama --model llama3.2

use crate::providers::{self, Message, ProviderKind, Role};
use crate::utils::config::Config;
use crate::utils::history::{self, Session};
use crate::utils::tokens::{estimate_tokens, format_cost, TokenPricing};
use anyhow::{bail, Result};
use clap::Args;
use colored::Colorize;
use std::io::{self, BufRead, Write};

#[derive(Args, Debug)]
pub struct ChatArgs {
    /// LLM provider to use (openai, anthropic, gemini, ollama)
    #[arg(short, long, default_value = "gemini")]
    pub provider: String,

    /// Model override (uses provider default if not set)
    #[arg(short, long)]
    pub model: Option<String>,

    /// System prompt
    #[arg(short, long)]
    pub system: Option<String>,

    /// Skip saving this session to history
    #[arg(long)]
    pub no_save: bool,
}

pub async fn run(args: ChatArgs) -> Result<()> {
    let config = Config::load()?;

    let kind = ProviderKind::from_str(&args.provider)
        .unwrap_or_else(|| {
            eprintln!("{} unknown provider '{}', falling back to Gemini",
                "warn:".yellow(), args.provider);
            ProviderKind::Gemini
        });

    let api_key = config
        .get_key(&args.provider.to_lowercase())
        .or_else(|| config.get_key(&kind.to_string().to_lowercase()))
        .map(String::from);

    let model = args.model
        .or_else(|| config.get_model(&kind.to_string().to_lowercase()).map(String::from));

    let provider = providers::build(kind, api_key, model.clone());

    let mut session = Session::new(kind, provider.model().to_string());
    let mut messages: Vec<Message> = Vec::new();

    // Inject system prompt if provided
    if let Some(sys) = &args.system {
        messages.push(Message {
            role: Role::System,
            content: sys.clone(),
        });
    }

    print_chat_header(kind, provider.model());

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("  {} ", "you →".cyan().bold());
        stdout.flush()?;

        let mut input = String::new();
        if stdin.lock().read_line(&mut input)? == 0 {
            // EOF (Ctrl+D)
            break;
        }

        let input = input.trim().to_string();

        if input.is_empty() {
            continue;
        }

        // Built-in commands
        match input.as_str() {
            "/quit" | "/exit" | "/q" => break,
            "/clear" => {
                messages.retain(|m| m.role == Role::System);
                session = Session::new(kind, provider.model().to_string());
                println!("  {} conversation cleared\n", "→".dimmed());
                continue;
            }
            "/tokens" => {
                let total_text: String = messages.iter().map(|m| m.content.as_str()).collect();
                let est = estimate_tokens(&total_text);
                println!("  {} ~{est} tokens in context\n", "→".dimmed());
                continue;
            }
            "/cost" => {
                if let Some(pricing) = TokenPricing::for_provider(kind, provider.model()) {
                    let total_text: String = messages.iter().map(|m| m.content.as_str()).collect();
                    let est = estimate_tokens(&total_text) as u32;
                    let cost = pricing.estimate_cost(est, est / 2);
                    println!("  {} estimated session cost: {}\n", "→".dimmed(), format_cost(cost).green());
                } else {
                    println!("  {} no pricing data for this provider (local model?)\n", "→".dimmed());
                }
                continue;
            }
            "/help" => {
                print_chat_help();
                continue;
            }
            _ => {}
        }

        messages.push(Message {
            role: Role::User,
            content: input.clone(),
        });
        session.push(Message { role: Role::User, content: input });

        print!("\n  {} ", format!("{} →", kind).color(kind.color_tag()).bold());
        stdout.flush()?;

        match provider.complete(&messages).await {
            Ok(resp) => {
                // Word-wrap the response at 80 chars
                let wrapped = wrap_text(&resp.content, 76);
                println!("{}\n", wrapped.white());

                let stats = format!(
                    "{}ms{}",
                    resp.latency_ms,
                    resp.tokens_used
                        .map(|t| format!(" · {t} tokens"))
                        .unwrap_or_default(),
                );
                println!("  {}\n", stats.dimmed());

                session.record_turn(resp.tokens_used, resp.latency_ms);

                let reply = Message {
                    role: Role::Assistant,
                    content: resp.content.clone(),
                };
                messages.push(reply.clone());
                session.push(reply);
            }
            Err(e) => {
                eprintln!("\n  {} {}\n", "error:".red().bold(), e);
                // Pop the user message so the turn can be retried
                messages.pop();
                session.messages.pop();
            }
        }
    }

    println!("\n  {} {} turns · {} tokens",
        "session ended →".dimmed(),
        session.turns(),
        session.total_tokens,
    );

    if !args.no_save {
        history::save_session(&session)?;
    }

    Ok(())
}

fn print_chat_header(kind: ProviderKind, model: &str) {
    println!("  {} {} {}",
        "chat".bold(),
        "·".dimmed(),
        format!("{kind} / {model}").color(kind.color_tag()),
    );
    println!("  {}", "type /help for commands, Ctrl+D to quit".dimmed());
    println!();
}

fn print_chat_help() {
    println!();
    println!("  {}", "chat commands".bold());
    println!("  {}  show this help", "/help".cyan());
    println!("  {}  clear conversation history", "/clear".cyan());
    println!("  {}  estimate tokens in context", "/tokens".cyan());
    println!("  {}  estimate session cost", "/cost".cyan());
    println!("  {}  quit", "/quit".cyan());
    println!();
}

fn wrap_text(text: &str, width: usize) -> String {
    text.lines()
        .map(|line| {
            if line.len() <= width {
                return line.to_string();
            }
            let mut result = String::new();
            let mut current_len = 0;
            for word in line.split_whitespace() {
                if current_len + word.len() + 1 > width && current_len > 0 {
                    result.push('\n');
                    result.push_str("  ");
                    current_len = 2;
                } else if current_len > 0 {
                    result.push(' ');
                    current_len += 1;
                }
                result.push_str(word);
                current_len += word.len();
            }
            result
        })
        .collect::<Vec<_>>()
        .join("\n")
}
