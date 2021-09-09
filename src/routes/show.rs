use crate::common::AppData;
use crate::config::PackageConfig;
use crate::request::PackageFileRequest;
use actix_files::NamedFile;
use actix_web::{
    error::{ErrorBadRequest, ErrorInternalServerError, ErrorNotFound},
    get,
    web::Data,
    HttpRequest, HttpResponse, Result as ActixResult,
};
use log::debug;
use std::{fs::create_dir_all, path::PathBuf};

fn get_unpacked_root_directory(config: &PackageConfig, request: &PackageFileRequest) -> PathBuf {
    let mut path = PathBuf::new();

    path.push(".");
    path.push(".tmp");
    path.push(config.key());
    path.push(&request.version);

    path
}

fn get_unpacked_package_directory(unpacked_root_directory: &PathBuf) -> PathBuf {
    let mut path = unpacked_root_directory.clone();

    path.push("package");

    path
}

fn get_unpacked_file_path(
    unpacked_package_directory: &PathBuf,
    request: &PackageFileRequest,
) -> PathBuf {
    let mut path = unpacked_package_directory.clone();

    let requested_file_name = if request.path == "" {
        "index.html"
    } else {
        &request.path
    };

    path.push(requested_file_name);

    path
}

#[get("/show/{tail:.*}")]
pub async fn show_handler(
    req: HttpRequest,
    app_data: Data<AppData<'_>>,
) -> ActixResult<HttpResponse> {
    let path = req.match_info().query("tail").parse::<String>()?;
    let request: PackageFileRequest = path
        .parse()
        .map_err(|_| ErrorBadRequest("failed to parse package request"))?;

    debug!("Client requested: {}", path);

    if request.path == "" && !path.ends_with("/") {
        let mut target = String::from("/show/");
        target.push_str(&path);
        target.push_str("/");

        return Ok(HttpResponse::MovedPermanently()
            .header("location", target)
            .finish());
    }

    let package_config = app_data
        .config
        .get_package(&request.name)
        .ok_or(ErrorNotFound("not found"))?;

    let unpacked_root_directory = get_unpacked_root_directory(package_config, &request);
    let unpacked_package_directory = get_unpacked_package_directory(&unpacked_root_directory);
    let unpacked_file_path = get_unpacked_file_path(&unpacked_package_directory, &request);

    if !unpacked_package_directory.exists() {
        let info = app_data
            .cache
            .update(package_config)
            .map_err(|_| ErrorInternalServerError("failed to update cache"))?;
        let tarball_url = info
            .manifest
            .get_tarball_url(&request.version)
            .ok_or(ErrorNotFound(
                "the requested version was not found in the registry",
            ))?;
        create_dir_all(&unpacked_root_directory)?;
        tarball_url
            .download_and_unpack(&unpacked_root_directory, package_config)
            .map_err(|_| ErrorInternalServerError("failed to download and unpack the package"))?;
    }

    if !unpacked_file_path.exists() {
        let error_message = format!(
            "The requested file ({}) was not found",
            unpacked_file_path.to_str().ok_or(ErrorInternalServerError(
                "failed to extract unpacked file path to UTF-8 string"
            ))?
        );

        return Err(ErrorNotFound(error_message));
    }

    NamedFile::open(unpacked_file_path)?.into_response(&req)
}
