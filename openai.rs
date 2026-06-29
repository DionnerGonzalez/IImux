// providers/openai.rs — OpenAI Chat Completions API
//
// Docs: https://platform.openai.com/docs/api-reference/chat

use super::{CompletionResponse, LlmProvider, Message, ProviderKind, Role};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Instant;

pub struct OpenAiProvider {
    api_key: String,
    model: String,
    client: Client,
}

impl OpenAiProvider {
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
struct OpenAiRequest<'a> {
    model: &'a str,
    messages: Vec<OpenAiMessage<'a>>,
    max_tokens: u32,
}

#[derive(Serialize)]
struct OpenAiMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
    model: Option<String>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Deserialize)]
struct ChoiceMessage {
    content: String,
}

#[derive(Deserialize)]
struct Usage {
    total_tokens: u32,
}

// --- trait impl ---

#[async_trait]
impl LlmProvider for OpenAiProvider {
    fn kind(&self)  -> ProviderKind { ProviderKind::OpenAI }
    fn model(&self) -> &str         { &self.model }

    async fn complete(&self, messages: &[Message]) -> Result<CompletionResponse> {
        let oai_messages: Vec<OpenAiMessage> = messages
            .iter()
            .map(|m| OpenAiMessage {
                role: match m.role {
                    Role::System    => "system",
                    Role::User      => "user",
                    Role::Assistant => "assistant",
                },
                content: &m.content,
            })
            .collect();

        let body = OpenAiRequest {
            model: &self.model,
            messages: oai_messages,
            max_tokens: 2048,
        };

        let start = Instant::now();

        let resp = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        let latency_ms = start.elapsed().as_millis();

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("OpenAI error {status}: {text}"));
        }

        let data: OpenAiResponse = resp.json().await?;
        let content = data
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .unwrap_or_default();

        Ok(CompletionResponse {
            content,
            model: data.model.unwrap_or_else(|| self.model.clone()),
            provider: ProviderKind::OpenAI,
            tokens_used: data.usage.map(|u| u.total_tokens),
            latency_ms,
        })
    }
}
