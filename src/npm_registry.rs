use crate::config::PackageConfig;
use crate::error::NpmPackageServerError;
use chrono::{DateTime, Utc};
use flate2::bufread::GzDecoder;
use log::debug;
use reqwest::blocking::Client;
use reqwest::header;
use reqwest::Url;
use semver::{Prerelease, Version};
use serde::Deserialize;
use serde_json::from_str;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;
use tar::Archive;
use urlencoding::encode;

#[derive(Debug, Clone, Deserialize)]
pub struct TarballUrl(String);

#[derive(Debug, Clone, Deserialize)]
struct DistInfo {
    tarball: TarballUrl,
}

#[derive(Debug, Clone, Deserialize)]
struct VersionInfo {
    dist: DistInfo,
    name: String,
    version: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PackageManifest {
    versions: HashMap<String, VersionInfo>,
    time: HashMap<String, String>,
}

impl PackageConfig {
    pub fn fetch_manifest(&self) -> Result<PackageManifest, NpmPackageServerError> {
        let client = Client::builder()
            .danger_accept_invalid_certs(!self.ssl_verify)
            .build()?;

        let full_package_url = format!("{}{}", self.registry, encode(&self.name));
        let url = Url::parse(&full_package_url)?;
        let mut builder = client.get(url);

        match self.access_token.as_ref() {
            Some(access_token) => {
                builder = builder.header(header::AUTHORIZATION, format!("Bearer {}", access_token));
            }
            None => {}
        };

        debug!("downloading");
        let response = builder.send()?;
        debug!("content-length: {:#?}", response.content_length());

        if response.status() != 200 {
            return Err(NpmPackageServerError::PackageNotFound(format!(
                "package {} was not found on {}",
                self.name, self.registry
            )));
        }

        let response_text = response.text()?;
        debug!("parsing {} bytes of text", response_text.len());
        let deserialized: PackageManifest = from_str(&response_text)?;
        debug!("done!");

        Ok(deserialized)
    }
}

impl PackageManifest {
    pub fn get_tarball_url(&self, version: &str) -> Option<TarballUrl> {
        self.versions
            .get(version)
            .map(|version| version.dist.tarball.clone())
    }
}

impl TarballUrl {
    pub fn download_and_unpack<P: AsRef<Path>>(
        &self,
        destination_dir: P,
        package_config: &PackageConfig,
    ) -> Result<(), NpmPackageServerError> {
        let client = Client::builder()
            .danger_accept_invalid_certs(!package_config.ssl_verify)
            .build()?;

        let parsed_url = Url::parse(self.0.as_str())?;
        let mut builder = client.get(parsed_url);

        match package_config.access_token.as_ref() {
            Some(access_token) => {
                builder = builder.header(header::AUTHORIZATION, format!("Bearer {}", access_token));
            }
            None => {}
        };

        let response = builder.send()?;

        let buf_reader = BufReader::new(response);
        let deflater = GzDecoder::new(buf_reader);
        let mut archive = Archive::new(deflater);

        debug!("unpacking...");
        archive.unpack(destination_dir)?;
        debug!("done!");

        Ok(())
    }
}

#[derive(Debug)]
pub struct VersionInfoForTemplates {
    pub version: String,
    pub parsed_version: Version,
    pub published: Option<DateTime<Utc>>,
}

pub type ManifestForTemplates = Vec<VersionInfoForTemplates>;

#[derive(Debug)]
pub struct CachedManifest {
    pub updated: Instant,
    pub manifest: Arc<PackageManifest>,
    pub manifest_for_templates: Arc<ManifestForTemplates>,
}

pub struct CachedManifestRepository {
    inner: Mutex<HashMap<String, Arc<CachedManifest>>>,
}

impl CachedManifestRepository {
    pub fn new() -> Self {
        CachedManifestRepository {
            inner: Mutex::new(HashMap::new()),
        }
    }

    pub fn update(
        &self,
        package_config: &PackageConfig,
    ) -> Result<Arc<CachedManifest>, NpmPackageServerError> {
        let key = package_config.key();

        let mut inner = self.inner.lock()?;

        let a = match inner.entry(key.clone()) {
            Entry::Occupied(entry) => {
                let now = Instant::now();
                let e = entry.get();
                let base = e.updated;

                if now - base >= Duration::from_millis(15000) {
                    None
                } else {
                    Some(Arc::clone(e))
                }
            }
            Entry::Vacant(_entry) => None,
        };

        if let Some(b) = a {
            return Ok(Arc::clone(&b));
        }

        let manifest = package_config.fetch_manifest()?;
        let manifest_for_templates = get_manifest_for_templates(&manifest)?;

        let cached_entry = Arc::new(CachedManifest {
            updated: Instant::now(),
            manifest: Arc::new(manifest),
            manifest_for_templates: Arc::new(manifest_for_templates),
        });

        inner.insert(key.clone(), Arc::clone(&cached_entry));

        Ok(Arc::clone(&cached_entry))
    }
}

fn get_manifest_for_templates(
    package_manifest: &PackageManifest,
) -> Result<ManifestForTemplates, NpmPackageServerError> {
    let mut result = package_manifest
        .versions
        .iter()
        .filter_map(|(version_id, _)| {
            let parsed_version = Version::parse(version_id).ok()?;

            if parsed_version.pre != Prerelease::EMPTY {
                return None;
            }

            Some(VersionInfoForTemplates {
                version: version_id.clone(),
                parsed_version,
                published: {
                    let source = package_manifest.time.get(version_id);

                    if let Some(source) = source {
                        source.parse::<DateTime<Utc>>().ok()
                    } else {
                        None
                    }
                },
            })
        })
        .collect::<ManifestForTemplates>();

    result.sort_by(|a, b| b.parsed_version.cmp(&a.parsed_version));

    Ok(result)
}
