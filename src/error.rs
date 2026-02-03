use thiserror::Error;

#[derive(Error, Debug)]
pub enum MedullaError {
    #[error("Not in a medulla project. Run 'medulla init' first.")]
    NotInitialized,

    #[error("Already initialized. Remove .medulla/ to reinitialize.")]
    AlreadyInitialized,

    #[error("Entity not found: {0}")]
    EntityNotFound(String),

    #[error("Invalid entity type: {0}")]
    InvalidEntityType(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Loro error: {0}")]
    Loro(#[from] loro::LoroError),

    #[error("Loro encode error: {0}")]
    LoroEncode(#[from] loro::LoroEncodeError),

    #[error("Embedding error: {0}")]
    Embedding(String),
}

pub type Result<T> = std::result::Result<T, MedullaError>;
