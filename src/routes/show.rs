use crate::error::PackageTrackingError;
use crate::npm_registry::DownloadManager;
use crate::request::PackageFileRequest;
use crate::{common::AppData, error::NpmPackageServerError};
use log::debug;
use rouille::{extension_to_mime, Response};
use std::sync::Arc;
use std::{ffi::OsStr, fs::File, path::Path};

fn get_extension_from_filename(filename: Option<&str>) -> Option<&str> {
    filename
        .map(|filename| Path::new(filename).extension().and_then(OsStr::to_str))
        .flatten()
}

pub fn show_handler(
    app_data: Arc<AppData>,
    path: String,
) -> Result<Response, NpmPackageServerError> {
    let request: PackageFileRequest = path.parse()?;

    debug!("Client requested: {}", path);

    if request.path == "" && !path.ends_with("/") {
        let mut target = String::from("/show/");
        target.push_str(&path);
        target.push_str("/");

        return Ok(Response::redirect_301(target));
    }

    let package_config = app_data
        .config
        .get_package(&request.name)
        .ok_or_else(|| PackageTrackingError::PackageIsNotTracked(request.name.clone()))?;

    let download_paths = DownloadManager::get_download_paths(package_config, &request);

    if !download_paths.package_directory.exists() {
        let info = app_data.manifest_repository.get_manifest(package_config)?;
        let tarball_url = info.get_tarball_url(&request.version).ok_or_else(|| {
            NpmPackageServerError::Registry(format!(
                "the specified version \"{}\" does not exist in \"{}\"",
                &request.version, info.registry_url
            ))
        })?;

        app_data.download_manager.download(
            package_config,
            tarball_url,
            download_paths.root_directory,
        )?;
    }

    if !download_paths.requested_file_path.exists() {
        return Err(NpmPackageServerError::NoSuchFile(request.path));
    }

    let str = download_paths
        .requested_file_path
        .to_str()
        .ok_or_else(|| NpmPackageServerError::Generic(String::from("failed to convert path")))?
        .to_owned();

    let extension = get_extension_from_filename(Some(&str));

    let mime = extension
        .map(|extension| {
            if extension == "md" {
                return "text/markdown; charset=utf-8";
            }

            return extension_to_mime(extension);
        })
        .unwrap_or("text/html");

    let used_mime = if mime == "application/octet-stream" {
        "text/html"
    } else {
        mime
    };

    let file = File::open(download_paths.requested_file_path)?;

    Ok(Response::from_file(used_mime, file))
}
