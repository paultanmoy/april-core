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
}

#[derive(Clone, Serialize)]
#[serde(tag = "type")]
pub enum Message {
    Text { text: String },
    Image(Image),
}

mod assistant;
pub use assistant::Assistant;

pub mod model;