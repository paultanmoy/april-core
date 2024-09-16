use std::future::Future;

use super::{Error, Image, Message};

#[derive(Debug)]
pub struct LanguageModelPrompt {
    max_tokens: usize,
    messages: Vec<Message>,
    temperature: f32,
    stop_sequences: Vec<String>,
    system: Option<String>,
}

impl From<Image> for LanguageModelPrompt {
    fn from(value: Image) -> Self {
        Self {
            max_tokens: 1024,
            messages: vec![value.into()],
            temperature: 0.63,
            stop_sequences: Vec::new(),
            system: None,
        }
    }
}

impl From<String> for LanguageModelPrompt {
    fn from(value: String) -> Self {
        Self {
            max_tokens: 1024,
            messages: vec![value.into()],
            temperature: 0.63,
            stop_sequences: Vec::new(),
            system: None,
        }
    }
}

impl From<&str> for LanguageModelPrompt {
    fn from(value: &str) -> Self {
        value.to_string().into()
    }
}

impl LanguageModelPrompt {
    pub fn add_message(self, message: impl Into<Message>) -> Self {
        let mut messages = self.messages;
        messages.push(message.into());

        Self {
            messages,
            ..self
        }
    }
    pub fn max_tokens(self, max_tokens: usize) -> Self {
        Self {
            max_tokens,
            ..self
        }
    }

    pub fn temperature(self, temperature: f32) -> Self {
        Self {
            temperature,
            ..self
        }
    }

    pub fn stop_sequence(self, stop_sequence: impl Into<String>) -> Self {
        let mut stop_sequences = self.stop_sequences;
        stop_sequences.push(stop_sequence.into());

        Self {
            stop_sequences,
            ..self
        }
    }

    pub fn system(self, system: impl Into<String>) -> Self {
        Self {
            system: Some(system.into()),
            ..self
        }
    }
}

pub trait LanguageModel {
    fn inference(&self, prompt: LanguageModelPrompt) -> impl Future<Output = Result<Message, Error>>;
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