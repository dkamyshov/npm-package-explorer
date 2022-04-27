use crate::cache::CachingError;
use crate::coalescer::CoalescingError;
use crate::request::PackageFileRequestParsingError;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum ManifestFetchError {
    #[error("underlying request error: {0}")]
    UnderlyingRequestError(String),
    #[error("failed to parse url: {0}")]
    UrlParseError(#[from] url::ParseError),
    #[error("package {name} doesn't exist in {registry}")]
    PackageDoesNotExistError { registry: String, name: String },
    #[error("deserialization error: {0}")]
    ResponseDeserializationError(String),
}

impl From<reqwest::Error> for ManifestFetchError {
    fn from(value: reqwest::Error) -> Self {
        ManifestFetchError::UnderlyingRequestError(value.to_string())
    }
}

impl From<serde_json::Error> for ManifestFetchError {
    fn from(value: serde_json::Error) -> Self {
        ManifestFetchError::ResponseDeserializationError(value.to_string())
    }
}

#[derive(Error, Debug, Clone)]
pub enum TarballDownloadError {
    #[error("underlying request error: {0}")]
    UnderlyingRequestError(String),
    #[error("failed to parse url: {0}")]
    UrlParseError(#[from] url::ParseError),
    #[error("io error: {0}")]
    IoError(String),
}

impl From<reqwest::Error> for TarballDownloadError {
    fn from(value: reqwest::Error) -> Self {
        TarballDownloadError::UnderlyingRequestError(value.to_string())
    }
}

impl From<std::io::Error> for TarballDownloadError {
    fn from(value: std::io::Error) -> Self {
        TarballDownloadError::IoError(value.to_string())
    }
}

#[derive(Error, Debug, Clone)]
pub enum PackageTrackingError {
    #[error("no tracked packages")]
    NoTrackedPackages,
    #[error("package \"{0}\" is not tracked")]
    PackageIsNotTracked(String),
    #[error("no versions known for package \"{0}\"")]
    NoVersions(String),
}

#[derive(Error, Debug, Clone)]
pub enum NpmPackageServerError {
    #[error("the package name is not specified")]
    PackageNameIsNotSpecified,
    #[error("the requested file does not exist in the package: {0}")]
    NoSuchFile(String),
    #[error("failed to render a template: {0}")]
    TemplateRendering(String),
    #[error("failed to render badge: {0}")]
    BadgeRendering(#[from] std::fmt::Error),
    #[error("npm registry error: {0}")]
    Registry(String),
    #[error("generic error: {0}")]
    Generic(String),
    #[error("io error: {0}")]
    IoError(String),
    #[error("failed to parse config: {0}")]
    ConfigDeserializeError(#[from] toml::de::Error),
    #[error("synchronization error: {0}")]
    SyncError(String),
    #[error("request coalescing error: {0}")]
    CoalescingError(#[from] CoalescingError),
    #[error("manifest fetch error: {0}")]
    ManifestFetchError(#[from] ManifestFetchError),
    #[error("couldn't download tarball: {0}")]
    TarballDownloadError(#[from] TarballDownloadError),
    #[error("cache error: {0}")]
    CachingError(#[from] CachingError),
    #[error("failed to parse package file request: {0}")]
    PackageFileRequestParsingError(#[from] PackageFileRequestParsingError),
    #[error("package tracking error: {0}")]
    PackageTrackingError(#[from] PackageTrackingError),
    #[error("serde error: {0}")]
    SerdeError(String),
}

impl From<serde_json::Error> for NpmPackageServerError {
    fn from(value: serde_json::Error) -> Self {
        NpmPackageServerError::SerdeError(value.to_string())
    }
}

impl From<handlebars::RenderError> for NpmPackageServerError {
    fn from(value: handlebars::RenderError) -> Self {
        NpmPackageServerError::TemplateRendering(value.to_string())
    }
}

impl From<std::io::Error> for NpmPackageServerError {
    fn from(value: std::io::Error) -> Self {
        NpmPackageServerError::IoError(value.to_string())
    }
}

impl<T> From<std::sync::PoisonError<T>> for NpmPackageServerError {
    fn from(error: std::sync::PoisonError<T>) -> Self {
        NpmPackageServerError::SyncError(error.to_string())
    }
}
