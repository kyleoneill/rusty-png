use thiserror::Error;

#[derive(Error, Debug)]
pub enum DecodeError {
    #[error("Bad file path: {0}")]
    BadFilePath(String),
    #[error("Failed to open file with path: {0}")]
    FailedToOpenFile(String),
    #[error("Failed to read file with path: {0}")]
    FailedToReadFile(String),
    #[error("PNG file has an invalid signature")]
    InvalidSignature(),
    #[error("The PNG file has an invalid file structure")]
    InvalidStructure()
}
