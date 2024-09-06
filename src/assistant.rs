use std::future::Future;

use anyhow::Error;
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Serialize)]
#[serde(tag = "type")]
pub enum Message {
    Text { text: String },
    Image { image: Vec<u8> },
}

pub trait Assistant {
    fn solve(&self, query: &str, context: Option<Value>) -> impl Future<Output = Result<(Value, Option<Message>), Error>> + std::marker::Send;
}