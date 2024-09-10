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

mod assistant;
pub use assistant::{Assistant, AssistantResponse};

pub mod model;

#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum LanguageModel {
    Anthropic(model::anthropic::AnthropicModel),
}