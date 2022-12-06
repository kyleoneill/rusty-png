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
    InvalidStructure(),
    #[error("Unsupported Feature: {0}")]
    UnsupportedFeature(String),
    #[error("A chunk checksum has failed validation")]
    FailedChecksum(),
    #[error("Failed to decode PNG data")]
    FailedDecoding(),
    #[error("The PNG header is invalid, it should be 25 bytes long and have the chunk type of 'IHDR'")]
    InvalidHeader()
}
