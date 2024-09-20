use std::fmt;

use base64::prelude::{BASE64_STANDARD, Engine as _};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize)]
pub struct Image {
    media_type: String,
    data: Vec<u8>,
}

impl fmt::Display for Image {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "data:{};base64, {}", self.media_type(), BASE64_STANDARD.encode(self.data()))
    }
}

impl Image {
    #[inline]
    pub fn new(media_type: impl Into<String>, data: Vec<u8>) -> Self {
        Self { media_type: media_type.into(), data }
    }

    #[inline]
    pub fn media_type(&self) -> &str {
        &self.media_type
    }

    #[inline]
    pub fn data(&self) -> Vec<u8> {
        self.data.clone()
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type")]
pub enum Message {
    #[serde(rename = "image")]
    Image(Image),

    #[serde(rename = "text")]
    Text { text: String },
}

impl fmt::Display for Message {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Message::Image(image) => write!(f, "{}", image),
            Message::Text { text } => f.write_str(text.as_str()),
        }
    }
}

impl From<Image> for Message {
    fn from(value: Image) -> Self {
        Self::Image(value)
    }
}

impl From<String> for Message {
    fn from(value: String) -> Self {
        Self::Text { text: value }
    }
}

impl From<&str> for Message {
    fn from(value: &str) -> Self {
        value.to_string().into()
    }
}

mod assistant;
pub use assistant::{Assistant, AssistantResponse};

mod error;
pub use error::Error;

pub mod model;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "provider")]
pub enum LanguageModel {
    Anthropic(model::anthropic::AnthropicModel),
}

impl model::LanguageModel for LanguageModel {
    async fn inference(&self, prompt: model::LanguageModelPrompt) -> Result<Message, Error> {
        match self {
            Self::Anthropic(model) => model,
        }.inference(prompt).await
    }
}

impl LanguageModel {
    pub fn anthropic(api_key: impl Into<String>, api_version: impl Into<String>, model: impl Into<String>) -> Self {
        Self::Anthropic(model::anthropic::AnthropicModel::new(api_key, api_version, model))
    }

    #[cfg(feature = "aws-bedrock")]
    pub async fn anthropic_bedrock(api_version: impl Into<String>, model: impl Into<String>, aws_config: Option<model::AwsConfig>) -> Self {
        Self::Anthropic(model::anthropic::AnthropicModel::bedrock(api_version, model, aws_config).await)
    }
}