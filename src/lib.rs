use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Image {
    media_type: String,
    data: Vec<u8>,
}

impl Image {
    #[inline]
    pub fn new(media_type: impl Into<String>, data: Vec<u8>) -> Self {
        Self { media_type: media_type.into(), data }
    }

    pub fn media_type(&self) -> &str {
        &self.media_type
    }

    pub fn data(&self) -> Vec<u8> {
        self.data.clone()
    }
}

#[derive(Clone, Serialize)]
#[serde(tag = "type")]
pub enum Message {
    #[serde(rename = "text")]
    Text { text: String },

    #[serde(rename = "image")]
    Image(Image),
}

mod assistant;
pub use assistant::{Assistant, AssistantResponse};

pub mod model;