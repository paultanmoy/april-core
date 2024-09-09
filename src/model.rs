use std::future::Future;

use anyhow::Error;

use super::{Image, Message};

pub trait LanguageModel {
    fn inference(&self, prompt: &str, image: Option<Image>) -> impl Future<Output = Result<Message, Error>>;
}

pub mod anthropic;

#[cfg(feature = "aws-bedrock")]
mod bedrock;

pub mod cohere;
pub mod meta;
pub mod mistral;
pub mod openai;
pub mod stability;