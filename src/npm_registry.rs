use crate::cache::Cache;
use crate::coalescer::Coalescer;
use crate::config::PackageConfig;
use crate::error::ManifestFetchError;
use crate::error::NpmPackageServerError;
use crate::error::TarballDownloadError;
use crate::request::PackageFileRequest;
use chrono::{DateTime, Utc};
use flate2::bufread::GzDecoder;
use log::debug;
use reqwest::blocking::Client;
use reqwest::header;
use reqwest::Url;
use semver::{Prerelease, Version};
use serde::Deserialize;
use serde_json::from_str;
use std::collections::HashMap;
use std::fs::create_dir_all;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tar::Archive;
use urlencoding::encode;

#[derive(Debug, Clone, Deserialize)]
pub struct TarballUrl(String);

#[derive(Debug, Clone, Deserialize)]
struct NpmDistInfo {
    tarball: TarballUrl,
}

#[derive(Debug, Clone, Deserialize)]
struct NpmVersionInfo {
    dist: NpmDistInfo,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NpmPackageManifest {
    versions: HashMap<String, NpmVersionInfo>,
    time: HashMap<String, String>,
}

#[derive(Clone)]
pub struct VersionManifest {
    version_str: String, // could this be removed?
    pub version: Version,
    pub published: DateTime<Utc>,
    pub tarball_url: TarballUrl,
}

#[derive(Clone)]
pub struct PackageManifest {
    pub versions: Vec<Arc<VersionManifest>>,
    pub registry_url: String,
    lookup: HashMap<String, Arc<VersionManifest>>,
}

pub struct ManifestRepository {
    cache: Cache<Arc<PackageManifest>>,
    coalescer: Coalescer<String, Result<Arc<PackageManifest>, NpmPackageServerError>>,
}

pub struct DownloadPaths {
    pub root_directory: PathBuf,
    pub package_directory: PathBuf,
    pub requested_file_path: PathBuf,
}

pub struct DownloadManager {
    coalescer: Coalescer<String, Result<(), TarballDownloadError>>,
}

fn fetch_manifest(
    package_config: &PackageConfig,
) -> Result<NpmPackageManifest, ManifestFetchError> {
    let client = Client::builder()
        .danger_accept_invalid_certs(!package_config.ssl_verify)
        .build()?;

    let full_package_url = format!(
        "{}{}",
        package_config.registry,
        encode(&package_config.name)
    );

    let url = Url::parse(&full_package_url)?;
    let mut builder = client.get(url.clone());

    match package_config.access_token.as_ref() {
        Some(access_token) => {
            debug!("adding authorization header");
            builder = builder.header(header::AUTHORIZATION, format!("Bearer {}", access_token));
        }
        None => {}
    };

    debug!("downloading {}", url);
    let response = builder.send()?;
    debug!("content-length: {:?}", response.content_length());

    if response.status() != 200 {
        return Err(ManifestFetchError::PackageDoesNotExistError {
            registry: package_config.registry.clone(),
            name: package_config.name.clone(),
        });
    }

    let response_text = response.text()?;
    debug!("parsing {} bytes of text", response_text.len());
    let deserialized: NpmPackageManifest = from_str(&response_text)?;
    debug!("done!");

    Ok(deserialized)
}

pub fn download_and_unpack_tarball<P: AsRef<Path>>(
    tarball_url: &TarballUrl,
    destination_dir: P,
    package_config: &PackageConfig,
) -> Result<(), TarballDownloadError> {
    create_dir_all(&destination_dir)?;

    let client = Client::builder()
        .danger_accept_invalid_certs(!package_config.ssl_verify)
        .build()?;

    let parsed_url = Url::parse(&tarball_url.to_string())?;
    let mut builder = client.get(parsed_url.clone());

    match package_config.access_token.as_ref() {
        Some(access_token) => {
            debug!("adding authorization header");
            builder = builder.header(header::AUTHORIZATION, format!("Bearer {}", access_token));
        }
        None => {}
    };

    debug!("downloading / unpacking {}", parsed_url);
    let response = builder.send()?;
    debug!("content-length: {:?}", response.content_length());

    let buf_reader = BufReader::new(response);
    let deflater = GzDecoder::new(buf_reader);
    let mut archive = Archive::new(deflater);

    archive.unpack(destination_dir)?;
    debug!("done!");

    Ok(())
}

impl TarballUrl {
    pub fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl PackageManifest {
    pub fn new(source: &NpmPackageManifest, registry_url: String) -> Self {
        let mut versions: Vec<VersionManifest> = source
            .versions
            .iter()
            .filter_map(|(version_id, _)| {
                let version_info = source.versions.get(version_id)?;
                let published = source.time.get(version_id)?.parse::<DateTime<Utc>>().ok()?;
                let parsed_version = Version::parse(version_id).ok()?;

                if parsed_version.pre != Prerelease::EMPTY {
                    return None;
                }

                Some(VersionManifest {
                    version_str: version_id.clone(),
                    version: parsed_version,
                    published,
                    tarball_url: version_info.dist.tarball.clone(),
                })
            })
            .collect();

        versions.sort_by(|a, b| b.version.cmp(&a.version));

        let mut result = PackageManifest {
            versions: vec![],
            registry_url,
            lookup: HashMap::new(),
        };

        for version in versions {
            let key = version.version_str.clone();
            let version_arc = Arc::new(version);
            result.lookup.insert(key, version_arc.clone());
            result.versions.push(version_arc);
        }

        result
    }

    pub fn get_tarball_url(&self, version: &str) -> Option<&TarballUrl> {
        self.lookup.get(version).map(|version| &version.tarball_url)
    }
}

impl ManifestRepository {
    pub fn new() -> Self {
        ManifestRepository {
            cache: Cache::new(Duration::from_millis(15000)),
            coalescer: Coalescer::new(),
        }
    }

    pub fn get_manifest(
        &self,
        package_config: &PackageConfig,
    ) -> Result<Arc<PackageManifest>, NpmPackageServerError> {
        let key = package_config.key();

        let coalesced_result = self.coalescer.execute(key.clone(), move || {
            let cached_entry = self.cache.get(&key, None)?;

            if let Some(entry) = cached_entry {
                return Ok(Arc::clone(&entry.value));
            }

            let manifest = fetch_manifest(package_config)?;
            let cached_entry = Arc::new(PackageManifest::new(
                &manifest,
                package_config.registry.clone(),
            ));

            self.cache.set(key, Arc::clone(&cached_entry))?;

            Ok(cached_entry)
        });

        match coalesced_result {
            Ok(result) => result,
            Err(err) => Err(err.into()),
        }
    }
}

impl DownloadManager {
    pub fn new() -> Self {
        DownloadManager {
            coalescer: Coalescer::new(),
        }
    }

    pub fn get_download_paths(
        config: &PackageConfig,
        request: &PackageFileRequest,
    ) -> DownloadPaths {
        let mut path = PathBuf::new();

        path.push(".");
        path.push(".tmp");
        path.push(config.identifier_safe_key());
        path.push(&request.version);

        let root_directory = path.clone();

        path.push("package");

        let package_directory = path.clone();

        let requested_file_name = if request.path == "" {
            &config.index_file
        } else {
            &request.path
        };

        path.push(requested_file_name);

        DownloadPaths {
            root_directory,
            package_directory,
            requested_file_path: path,
        }
    }

    pub fn download<P: AsRef<Path>>(
        &self,
        config: &PackageConfig,
        tarball_url: &TarballUrl,
        root_directory: P,
    ) -> Result<(), NpmPackageServerError> {
        let key = tarball_url.to_string();

        let coalesced_result = self.coalescer.execute(key.clone(), || {
            return download_and_unpack_tarball(tarball_url, root_directory, config);
        });

        match coalesced_result? {
            Ok(result) => Ok(result),
            Err(err) => Err(err.into()),
        }
    }
}
