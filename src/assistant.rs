use anyhow::Result;
use async_trait::async_trait;
use serde::Serialize;
use serde_json::Value;

use super::Message;

#[derive(Serialize)]
#[serde(untagged)]
pub enum AssistantResponse {
    Final { response: Message, #[serde(skip_serializing_if = "Option::is_none")] context: Option<Value> },
    Query { ask: String, #[serde(skip_serializing_if = "Option::is_none")] context: Option<Value> },
}

#[async_trait]
pub trait Assistant: Send + Sync {
    async fn solve(&self, query: &str, context: Option<Value>) -> Result<AssistantResponse>;
}