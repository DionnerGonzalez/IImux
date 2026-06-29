// utils/tokens.rs — rough token counting and cost estimation
//
// We use a simple byte-based heuristic: ~4 bytes per token for English,
// which matches GPT tokenizer output to within 10–15%.

use crate::providers::ProviderKind;

/// Estimate token count from a raw string.
/// Good enough for "how much will this cost?" questions.
pub fn estimate_tokens(text: &str) -> usize {
    // Heuristic: 1 token ≈ 4 chars for English prose
    // For code or multilingual content this can be off, but it's fast.
    let chars = text.chars().count();
    (chars / 4).max(1)
}

/// Cost per 1 million tokens in USD (as of mid-2025 pricing).
/// These are approximate — always check provider dashboards for current rates.
#[derive(Debug, Clone, Copy)]
pub struct TokenPricing {
    /// Cost per 1M input tokens
    pub input_per_million: f64,
    /// Cost per 1M output tokens
    pub output_per_million: f64,
}

impl TokenPricing {
    pub fn for_provider(provider: ProviderKind, model: &str) -> Option<Self> {
        match provider {
            ProviderKind::OpenAI => match model {
                m if m.contains("gpt-4o-mini") => Some(Self {
                    input_per_million: 0.15,
                    output_per_million: 0.60,
                }),
                m if m.contains("gpt-4o") => Some(Self {
                    input_per_million: 5.00,
                    output_per_million: 15.00,
                }),
                m if m.contains("gpt-4.1") => Some(Self {
                    input_per_million: 2.00,
                    output_per_million: 8.00,
                }),
                _ => Some(Self {
                    input_per_million: 5.00,
                    output_per_million: 15.00,
                }),
            },
            ProviderKind::Anthropic => match model {
                m if m.contains("haiku") => Some(Self {
                    input_per_million: 0.80,
                    output_per_million: 4.00,
                }),
                m if m.contains("sonnet") => Some(Self {
                    input_per_million: 3.00,
                    output_per_million: 15.00,
                }),
                m if m.contains("opus") => Some(Self {
                    input_per_million: 15.00,
                    output_per_million: 75.00,
                }),
                _ => Some(Self {
                    input_per_million: 3.00,
                    output_per_million: 15.00,
                }),
            },
            ProviderKind::Gemini => match model {
                m if m.contains("flash") => Some(Self {
                    input_per_million: 0.075,
                    output_per_million: 0.30,
                }),
                m if m.contains("pro") => Some(Self {
                    input_per_million: 1.25,
                    output_per_million: 5.00,
                }),
                // Gemma is open-weight; when self-hosted it's free
                _ => None,
            },
            // Ollama is local — cost is your electricity bill
            ProviderKind::Ollama => None,
        }
    }

    /// Estimate cost given token counts.
    pub fn estimate_cost(&self, input_tokens: u32, output_tokens: u32) -> f64 {
        let input_cost  = (input_tokens  as f64 / 1_000_000.0) * self.input_per_million;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * self.output_per_million;
        input_cost + output_cost
    }
}

/// Format a dollar amount to a readable string.
pub fn format_cost(usd: f64) -> String {
    if usd < 0.001 {
        format!("< $0.001")
    } else if usd < 0.01 {
        format!("${:.4}", usd)
    } else {
        format!("${:.3}", usd)
    }
}
