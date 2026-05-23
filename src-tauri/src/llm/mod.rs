pub mod embedder;
pub mod monologue;
pub mod opener;
pub mod openrouter;
pub mod prompts;
pub mod summarizer;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String, // "system" | "user" | "assistant"
    pub content: String,
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("no API key configured — fill api_key in config.local.json")]
    NoApiKey,
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("server returned {0}: {1}")]
    Server(u16, String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}
