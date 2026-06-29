// providers/gemini.rs — Google Gemini / Gemma via generativelanguage API
//
// Docs: https://ai.google.dev/api/generate-content

use super::{CompletionResponse, LlmProvider, Message, ProviderKind, Role};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;

pub struct GeminiProvider {
    api_key: String,
    model: String,
    client: Client,
}

impl GeminiProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: Client::new(),
        }
    }

    fn endpoint(&self) -> String {
        format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        )
    }
}

// --- request / response shapes ---

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(rename = "systemInstruction", skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiSystemInstruction>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct GeminiSystemInstruction {
    parts: Vec<GeminiPart>,
}

#[derive(Serialize, Deserialize, Clone)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Serialize, Deserialize, Clone)]
struct GeminiPart {
    text: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: u32,
    temperature: f32,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<UsageMetadata>,
}

#[derive(Deserialize)]
struct Candidate {
    content: GeminiContent,
}

#[derive(Deserialize)]
struct UsageMetadata {
    #[serde(rename = "totalTokenCount")]
    total_token_count: u32,
}

// --- trait impl ---

#[async_trait]
impl LlmProvider for GeminiProvider {
    fn kind(&self)  -> ProviderKind { ProviderKind::Gemini }
    fn model(&self) -> &str         { &self.model }

    async fn complete(&self, messages: &[Message]) -> Result<CompletionResponse> {
        let system_instruction = messages
            .iter()
            .find(|m| m.role == Role::System)
            .map(|m| GeminiSystemInstruction {
                parts: vec![GeminiPart { text: m.content.clone() }],
            });

        let contents: Vec<GeminiContent> = messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| GeminiContent {
                role: match m.role {
                    Role::User      => "user".to_string(),
                    Role::Assistant => "model".to_string(),
                    Role::System    => "user".to_string(),
                },
                parts: vec![GeminiPart { text: m.content.clone() }],
            })
            .collect();

        let body = GeminiRequest {
            contents,
            system_instruction,
            generation_config: GenerationConfig {
                max_output_tokens: 2048,
                temperature: 0.7,
            },
        };

        let start = Instant::now();

        let resp = self
            .client
            .post(self.endpoint())
            .json(&body)
            .send()
            .await?;

        let latency_ms = start.elapsed().as_millis();

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Gemini error {status}: {text}"));
        }

        let data: GeminiResponse = resp.json().await?;

        let content = data
            .candidates
            .into_iter()
            .next()
            .and_then(|c| c.content.parts.into_iter().next())
            .map(|p| p.text)
            .unwrap_or_default();

        Ok(CompletionResponse {
            content,
            model: self.model.clone(),
            provider: ProviderKind::Gemini,
            tokens_used: data.usage_metadata.map(|u| u.total_token_count),
            latency_ms,
        })
    }
}
