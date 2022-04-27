use crate::config::Config;
use crate::error::PackageTrackingError;
use crate::npm_registry::PackageManifest;
use crate::{common::AppData, error::NpmPackageServerError};
use chrono::Utc;
use log::debug;
use rouille::Response;
use serde_derive::Serialize;
use std::ops::Sub;
use std::sync::Arc;
use timeago::{languages, Formatter, Language};

#[derive(Serialize)]
struct TemplateVersion {
    version: String,
    time: String,
    formatted_time: String,
}

#[derive(Serialize)]
struct TemplatePackage {
    name: String,
    clear_name: String,
    versions: Vec<TemplateVersion>,
}

fn get_language_by_iso639_1_code(iso639_1: &str) -> Box<dyn Language + Send + Sync + 'static> {
    match iso639_1 {
        "ru" => Box::new(languages::russian::Russian),
        _ => Box::new(languages::english::English),
    }
}

fn transform_version_info_for_templates(
    config: &Config,
    source: Arc<PackageManifest>,
) -> Vec<TemplateVersion> {
    let language = get_language_by_iso639_1_code(
        config
            .timeago_language
            .as_ref()
            .unwrap_or(&String::from("en")),
    );

    let formatter = Formatter::with_language(language);
    let now = Utc::now();

    source
        .versions
        .iter()
        .map(|version| {
            let published = version.published.to_rfc3339();

            let published_ago = now
                .sub(version.published)
                .to_std()
                .map_or(String::from("unknown"), |d| formatter.convert(d));

            TemplateVersion {
                version: version.version.to_string(),
                time: published.clone(),
                formatted_time: published_ago,
            }
        })
        .collect()
}

pub fn index_handler(
    app_data: Arc<AppData>,
    package_name: Option<String>,
) -> Result<Response, NpmPackageServerError> {
    let first_package = app_data
        .config
        .get_first_package()
        .ok_or(PackageTrackingError::NoTrackedPackages)?;

    // TODO: check if this package exists
    let selected_package_name = package_name
        .clone()
        .unwrap_or(first_package.get_public_name().clone());

    let packages = app_data
        .config
        .packages
        .iter()
        .filter_map(|package_config| {
            let manifest = app_data
                .manifest_repository
                .get_manifest(package_config)
                .ok()?;

            let name = package_config.get_public_name();

            Some(TemplatePackage {
                name: name.clone(),
                clear_name: package_config.identifier_safe_key(),
                versions: transform_version_info_for_templates(
                    &app_data.config,
                    Arc::clone(&manifest),
                ),
            })
        })
        .collect::<Vec<TemplatePackage>>();

    debug!("selected_package_name: {}", selected_package_name);

    let data = json!({
        "packages": packages,
        "labels": app_data.config.labels,
        "selected_package_name": selected_package_name,
        "banner_gradient_left_color": app_data.config.banner_gradient_left_color,
        "banner_gradient_right_color": app_data.config.banner_gradient_right_color,
        "banner_color": app_data.config.banner_color
    });

    let body = app_data.handlebars.render("index", &data)?;

    Ok(Response::html(body))
}
