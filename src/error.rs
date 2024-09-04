#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
}