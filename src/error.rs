use thiserror::Error;

#[derive(Error, Debug)]
pub enum NpmPackageServerError {
    #[error("io error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("failed to parse config: {0}")]
    ConfigDeserializeError(#[from] toml::de::Error),
    #[error("failed to parse request: {0}")]
    PackageFileRequestParseError(String),
    #[error("synchronization error: {0}")]
    SyncError(String),
    #[error("failed to parse url: {0}")]
    UrlParseError(#[from] url::ParseError),
    #[error("generic request error: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("serde error: {0}")]
    SerdeError(#[from] serde_json::Error),
    #[error("package not found: {0}")]
    PackageNotFound(String),
}

impl<T> From<std::sync::PoisonError<T>> for NpmPackageServerError {
    fn from(error: std::sync::PoisonError<T>) -> Self {
        NpmPackageServerError::SyncError(error.to_string())
    }
}
