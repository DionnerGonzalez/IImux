// providers/ollama.rs — Ollama local model server
//
// Ollama exposes an OpenAI-compatible endpoint at localhost:11434.
// No API key needed — just have Ollama running and a model pulled.
//
// Docs: https://github.com/ollama/ollama/blob/main/docs/api.md

use super::{CompletionResponse, LlmProvider, Message, ProviderKind, Role};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;

pub struct OllamaProvider {
    model: String,
    client: Client,
    base_url: String,
}

impl OllamaProvider {
    pub fn new(model: String) -> Self {
        Self {
            model,
            client: Client::new(),
            base_url: "http://localhost:11434".to_string(),
        }
    }

    /// Allow pointing at a remote Ollama instance
    pub fn with_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }
}

// --- request / response shapes (OpenAI-compatible) ---

#[derive(Serialize)]
struct OllamaRequest<'a> {
    model: &'a str,
    messages: Vec<OllamaMessage<'a>>,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Serialize)]
struct OllamaMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct OllamaOptions {
    num_predict: i32,
    temperature: f32,
}

#[derive(Deserialize)]
struct OllamaResponse {
    message: OllamaMessageResponse,
    #[serde(rename = "eval_count")]
    eval_count: Option<u32>,
    #[serde(rename = "prompt_eval_count")]
    prompt_eval_count: Option<u32>,
}

#[derive(Deserialize)]
struct OllamaMessageResponse {
    content: String,
}

// --- trait impl ---

#[async_trait]
impl LlmProvider for OllamaProvider {
    fn kind(&self)  -> ProviderKind { ProviderKind::Ollama }
    fn model(&self) -> &str         { &self.model }

    async fn complete(&self, messages: &[Message]) -> Result<CompletionResponse> {
        let ollama_messages: Vec<OllamaMessage> = messages
            .iter()
            .map(|m| OllamaMessage {
                role: match m.role {
                    Role::System    => "system",
                    Role::User      => "user",
                    Role::Assistant => "assistant",
                },
                content: &m.content,
            })
            .collect();

        let body = OllamaRequest {
            model: &self.model,
            messages: ollama_messages,
            stream: false, // streaming handled separately by the TUI layer
            options: OllamaOptions {
                num_predict: 2048,
                temperature: 0.7,
            },
        };

        let start = Instant::now();

        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                // Give a friendlier error when Ollama isn't running
                if e.is_connect() {
                    anyhow!(
                        "Couldn't reach Ollama at {}.\n\
                         Make sure Ollama is running: `ollama serve`",
                        self.base_url
                    )
                } else {
                    e.into()
                }
            })?;

        let latency_ms = start.elapsed().as_millis();

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Ollama error {status}: {text}"));
        }

        let data: OllamaResponse = resp.json().await?;

        let tokens_used = match (data.prompt_eval_count, data.eval_count) {
            (Some(p), Some(e)) => Some(p + e),
            _ => None,
        };

        Ok(CompletionResponse {
            content: data.message.content,
            model: self.model.clone(),
            provider: ProviderKind::Ollama,
            tokens_used,
            latency_ms,
        })
    }
}
