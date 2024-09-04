use std::future::Future;

use serde_json::Value;

use super::error::Error;

#[derive(Clone)]
pub enum Message {
    Info { text: String },
    Query { ask: String },
}

pub trait Assistant {
    fn solve(&self, query: &str, context: Option<Value>) -> impl Future<Output = Result<(Value, Option<Message>), Error>> + std::marker::Send;
}