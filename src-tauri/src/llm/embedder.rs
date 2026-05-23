//! OpenAI embedding via OpenRouter.
//!
//! Model: `openai/text-embedding-3-small` (1536-dim, ~$0.02 / 1M tokens).
//! Per PRD §5.2, this is the chosen embedding model — fixed via config
//! `embedding_model` if users want to override.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::config::Config;

pub const EMBEDDING_DIM: usize = 1536;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const DEFAULT_MODEL: &str = "openai/text-embedding-3-small";

#[derive(Serialize)]
struct EmbeddingRequest<'a> {
    model: &'a str,
    input: &'a str,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

pub async fn embed(cfg: &Config, text: &str) -> Result<Vec<f32>, String> {
    if !cfg.is_usable() {
        return Err("no api_key for embedding".into());
    }
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("{}/embeddings", cfg.base_url.trim_end_matches('/'));
    let body = EmbeddingRequest {
        model: DEFAULT_MODEL,
        input: text,
    };
    let resp = client
        .post(&url)
        .bearer_auth(&cfg.api_key)
        .header("HTTP-Referer", "https://github.com/cipher/capybit")
        .header("X-Title", "Capybit")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("embedding server {status}: {text}"));
    }

    let parsed: EmbeddingResponse = resp.json().await.map_err(|e| e.to_string())?;
    let emb = parsed
        .data
        .into_iter()
        .next()
        .ok_or_else(|| "no embedding in response".to_string())?
        .embedding;

    if emb.len() != EMBEDDING_DIM {
        return Err(format!(
            "embedding dim mismatch: got {}, expected {EMBEDDING_DIM}",
            emb.len()
        ));
    }
    Ok(emb)
}
