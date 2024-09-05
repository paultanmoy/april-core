use std::future::Future;

use anyhow::Error;

pub trait LargeLanguageModel {
    fn inference(&self, prompt: &str) -> impl Future<Output = Result<&str, Error>>;
}

pub mod anthropic;
pub mod cohere;
pub mod meta;
pub mod mistral;
pub mod openai;
pub mod stability;