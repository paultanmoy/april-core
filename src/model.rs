use std::future::Future;

use super::{Error, Image, Message};

pub trait LanguageModel {
    fn inference(&self, prompt: &str, image: Option<Image>) -> impl Future<Output = Result<Message, Error>>;
}

pub mod anthropic;

#[cfg(feature = "aws-bedrock")]
mod bedrock;

#[cfg(feature = "aws-bedrock")]
pub use bedrock::AwsConfig;

pub mod cohere;
pub mod meta;
pub mod mistral;
pub mod openai;
pub mod stability;