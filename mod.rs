// providers/mod.rs — abstractions over LLM provider APIs
//
// Each provider implements the LlmProvider trait so the rest
// of the codebase doesn't need to know which API it's talking to.

pub mod anthropic;
pub mod gemini;
pub mod ollama;
pub mod openai;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

/// A single message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::System    => write!(f, "system"),
            Role::User      => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
        }
    }
}

/// The response returned after a single completion.
#[derive(Debug, Clone)]
pub struct CompletionResponse {
    pub content: String,
    pub model: String,
    pub provider: ProviderKind,
    /// Approximate tokens used (prompt + completion)
    pub tokens_used: Option<u32>,
    /// Wall-clock latency in milliseconds
    pub latency_ms: u128,
}

/// Supported providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProviderKind {
    OpenAI,
    Anthropic,
    Gemini,
    Ollama,
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderKind::OpenAI    => write!(f, "OpenAI"),
            ProviderKind::Anthropic => write!(f, "Anthropic"),
            ProviderKind::Gemini    => write!(f, "Gemini"),
            ProviderKind::Ollama    => write!(f, "Ollama"),
        }
    }
}

impl ProviderKind {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai" | "oai"       => Some(ProviderKind::OpenAI),
            "anthropic" | "claude" => Some(ProviderKind::Anthropic),
            "gemini" | "google"    => Some(ProviderKind::Gemini),
            "ollama" | "local"     => Some(ProviderKind::Ollama),
            _                      => None,
        }
    }

    pub fn default_model(&self) -> &'static str {
        match self {
            ProviderKind::OpenAI    => "gpt-4o",
            ProviderKind::Anthropic => "claude-sonnet-4-6",
            ProviderKind::Gemini    => "gemma-4-31b-it",
            ProviderKind::Ollama    => "llama3.2",
        }
    }

    pub fn color_tag(&self) -> colored::Color {
        match self {
            ProviderKind::OpenAI    => colored::Color::Green,
            ProviderKind::Anthropic => colored::Color::BrightYellow,
            ProviderKind::Gemini    => colored::Color::BrightBlue,
            ProviderKind::Ollama    => colored::Color::Magenta,
        }
    }
}

/// The trait every provider must implement.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn kind(&self) -> ProviderKind;
    fn model(&self) -> &str;

    async fn complete(&self, messages: &[Message]) -> Result<CompletionResponse>;
}

// Re-export constructors so callers don't need to know the module structure.
pub use anthropic::AnthropicProvider;
pub use gemini::GeminiProvider;
pub use ollama::OllamaProvider;
pub use openai::OpenAiProvider;

/// Build a provider from a kind + optional api key + optional model override.
pub fn build(
    kind: ProviderKind,
    api_key: Option<String>,
    model: Option<String>,
) -> Box<dyn LlmProvider> {
    let model = model.unwrap_or_else(|| kind.default_model().to_string());

    match kind {
        ProviderKind::OpenAI => Box::new(OpenAiProvider::new(
            api_key.unwrap_or_default(),
            model,
        )),
        ProviderKind::Anthropic => Box::new(AnthropicProvider::new(
            api_key.unwrap_or_default(),
            model,
        )),
        ProviderKind::Gemini => Box::new(GeminiProvider::new(
            api_key.unwrap_or_default(),
            model,
        )),
        ProviderKind::Ollama => Box::new(OllamaProvider::new(model)),
    }
}
