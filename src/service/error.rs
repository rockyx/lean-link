use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    /// IO Error
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),
}