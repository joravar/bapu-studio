//! AI Copilot client supporting local Ollama plus OpenAI & Anthropic BYOK.
//! API keys are read/written via `crate::secrets` (OS keychain), never saved in plaintext JSON.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiProvider {
    Ollama,
    OpenAi,
    Anthropic,
}

impl AiProvider {
    pub fn slug(&self) -> &'static str {
        match self {
            AiProvider::Ollama => "ollama",
            AiProvider::OpenAi => "openai",
            AiProvider::Anthropic => "anthropic",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            AiProvider::Ollama => "Ollama (Local)",
            AiProvider::OpenAi => "OpenAI (BYOK)",
            AiProvider::Anthropic => "Anthropic (BYOK)",
        }
    }

    pub fn default_model(&self) -> &'static str {
        match self {
            AiProvider::Ollama => "llama3.2",
            AiProvider::OpenAi => "gpt-4o-mini",
            AiProvider::Anthropic => "claude-3-5-sonnet-latest",
        }
    }

    pub fn default_base_url(&self) -> &'static str {
        match self {
            AiProvider::Ollama => "http://localhost:11434",
            AiProvider::OpenAi => "https://api.openai.com/v1",
            AiProvider::Anthropic => "https://api.anthropic.com/v1",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AiRequest {
    pub provider: AiProvider,
    pub base_url: String,
    pub model: String,
    pub api_key: String,
    pub system_prompt: String,
    pub user_prompt: String,
}

pub async fn generate(client: &reqwest::Client, req: &AiRequest) -> Result<String> {
    let base = req.base_url.trim_end_matches('/');
    match req.provider {
        AiProvider::Ollama => {
            let url = format!("{base}/api/chat");
            let body = json!({
                "model": req.model,
                "stream": false,
                "messages": [
                    { "role": "system", "content": req.system_prompt },
                    { "role": "user", "content": req.user_prompt }
                ]
            });
            let resp = client.post(&url).json(&body).send().await.context("calling Ollama")?;
            if !resp.status().is_success() {
                bail!("Ollama HTTP {}: {}", resp.status(), resp.text().await.unwrap_or_default());
            }
            let val: serde_json::Value = resp.json().await?;
            Ok(val
                .pointer("/message/content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string())
        }
        AiProvider::OpenAi => {
            if req.api_key.trim().is_empty() {
                bail!("OpenAI API key is required.");
            }
            let url = format!("{base}/chat/completions");
            let body = json!({
                "model": req.model,
                "messages": [
                    { "role": "system", "content": req.system_prompt },
                    { "role": "user", "content": req.user_prompt }
                ]
            });
            let resp = client
                .post(&url)
                .bearer_auth(req.api_key.trim())
                .json(&body)
                .send()
                .await
                .context("calling OpenAI")?;
            if !resp.status().is_success() {
                bail!("OpenAI HTTP {}: {}", resp.status(), resp.text().await.unwrap_or_default());
            }
            let val: serde_json::Value = resp.json().await?;
            Ok(val
                .pointer("/choices/0/message/content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string())
        }
        AiProvider::Anthropic => {
            if req.api_key.trim().is_empty() {
                bail!("Anthropic API key is required.");
            }
            let url = format!("{base}/messages");
            let body = json!({
                "model": req.model,
                "max_tokens": 1024,
                "system": req.system_prompt,
                "messages": [
                    { "role": "user", "content": req.user_prompt }
                ]
            });
            let resp = client
                .post(&url)
                .header("x-api-key", req.api_key.trim())
                .header("anthropic-version", "2023-06-01")
                .json(&body)
                .send()
                .await
                .context("calling Anthropic")?;
            if !resp.status().is_success() {
                bail!("Anthropic HTTP {}: {}", resp.status(), resp.text().await.unwrap_or_default());
            }
            let val: serde_json::Value = resp.json().await?;
            Ok(val
                .pointer("/content/0/text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string())
        }
    }
}
