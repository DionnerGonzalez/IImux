// providers/anthropic.rs — Anthropic Messages API
//
// Docs: https://docs.anthropic.com/en/api/messages

use super::{CompletionResponse, LlmProvider, Message, ProviderKind, Role};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;

pub struct AnthropicProvider {
    api_key: String,
    model: String,
    client: Client,
}

impl AnthropicProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: Client::new(),
        }
    }
}

// --- request / response shapes ---

#[derive(Serialize)]
struct AnthropicRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<&'a str>,
    messages: Vec<AnthropicMessage<'a>>,
}

#[derive(Serialize)]
struct AnthropicMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    model: String,
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    kind: String,
    text: Option<String>,
}

#[derive(Deserialize)]
struct Usage {
    input_tokens: u32,
    output_tokens: u32,
}

// --- trait impl ---

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn kind(&self)  -> ProviderKind { ProviderKind::Anthropic }
    fn model(&self) -> &str         { &self.model }

    async fn complete(&self, messages: &[Message]) -> Result<CompletionResponse> {
        // Anthropic separates the system prompt from the message list.
        let system = messages
            .iter()
            .find(|m| m.role == Role::System)
            .map(|m| m.content.as_str());

        let turns: Vec<AnthropicMessage> = messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| AnthropicMessage {
                role: match m.role {
                    Role::User      => "user",
                    Role::Assistant => "assistant",
                    Role::System    => "user", // already filtered, just satisfies exhaustiveness
                },
                content: &m.content,
            })
            .collect();

        let body = AnthropicRequest {
            model: &self.model,
            max_tokens: 2048,
            system,
            messages: turns,
        };

        let start = Instant::now();

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await?;

        let latency_ms = start.elapsed().as_millis();

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Anthropic error {status}: {text}"));
        }

        let data: AnthropicResponse = resp.json().await?;

        let content = data
            .content
            .into_iter()
            .filter(|b| b.kind == "text")
            .filter_map(|b| b.text)
            .collect::<Vec<_>>()
            .join("");

        let tokens_used = data
            .usage
            .map(|u| u.input_tokens + u.output_tokens);

        Ok(CompletionResponse {
            content,
            model: data.model,
            provider: ProviderKind::Anthropic,
            tokens_used,
            latency_ms,
        })
    }
}
