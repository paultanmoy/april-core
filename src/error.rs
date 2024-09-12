#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    ImageDecode(#[from] base64::DecodeError),

    #[error("{0}")]
    ModelResponse(String),

    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}