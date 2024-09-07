use std::future::Future;

use anyhow::Error;
use serde_json::Value;

use super::Message;

pub trait Assistant {
    fn solve(&self, query: &str, context: Option<Value>) -> impl Future<Output = Result<(Value, Option<Message>), Error>> + std::marker::Send;
}