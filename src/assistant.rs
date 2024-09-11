use async_trait::async_trait;
use serde::Serialize;
use serde_json::Value;
use tokio::sync::broadcast;

use super::Message;

#[derive(Serialize)]
#[serde(untagged)]
pub enum AssistantResponse {
    Final { response: Message, #[serde(skip_serializing_if = "Option::is_none")] context: Option<Value> },
    Query { ask: String, #[serde(skip_serializing_if = "Option::is_none")] context: Option<Value> },
}

#[async_trait]
#[typetag::serde(tag = "type")]
pub trait Assistant: Send + Sync {
    fn communicate(&mut self, #[allow(unused)] bx: broadcast::Sender<(String, Message)>) {}

    async fn solve(&self, query: &str, context: Option<Value>, session_id: &str) -> AssistantResponse;
}