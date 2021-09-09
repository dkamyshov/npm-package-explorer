use crate::common::AppData;
use crate::config::Config;
use crate::npm_registry::VersionInfoForTemplates;
use actix_web::error::{ErrorInternalServerError, ErrorNotFound};
use actix_web::web::{Data, Query};
use actix_web::{get, HttpResponse, Result as ActixResult};
use chrono::Utc;
use log::debug;
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
use std::ops::Sub;
use std::sync::Arc;
use timeago::{languages, Formatter, Language};
use urlencoding::encode;

#[derive(Deserialize, Debug)]
pub struct QueryParams {
    package: Option<String>,
}

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

fn filter_string(source: &str) -> String {
    let re: Regex = Regex::new(r"[^a-zA-Z0-9_]").unwrap();

    re.replace_all(encode(source).as_ref(), "_").to_string()
}

fn get_language_by_iso639_1_code(iso639_1: &str) -> Box<dyn Language + Send + Sync + 'static> {
    match iso639_1 {
        "ru" => Box::new(languages::russian::Russian),
        _ => Box::new(languages::english::English),
    }
}

fn transform_version_info_for_templates(
    config: &Config,
    source: Arc<Vec<VersionInfoForTemplates>>,
) -> Vec<TemplateVersion> {
    let language = get_language_by_iso639_1_code(
        config
            .timeago_language
            .as_ref()
            .unwrap_or(&String::from("en")),
    );

    let f = Formatter::with_language(language);
    let now = Utc::now();

    source
        .iter()
        .map(|version| {
            let published = version
                .published
                .map_or(String::from("unknown"), |t| t.to_rfc3339());

            let formatted_time = version.published.map_or(String::from("unknown"), |t| {
                now.sub(t)
                    .to_std()
                    .map_or(String::from("unknown"), |d| f.convert(d))
            });

            TemplateVersion {
                version: version.version.clone(),
                time: published.clone(),
                formatted_time,
            }
        })
        .collect()
}

#[get("/")]
pub async fn index_handler(
    query: Query<QueryParams>,
    app_data: Data<AppData<'_>>,
) -> ActixResult<HttpResponse> {
    let first_package = app_data
        .config
        .get_first_package()
        .ok_or(ErrorNotFound("there are 0 tracked packages"))?;

    let selected_package_name = query
        .package
        .clone()
        .unwrap_or(first_package.get_public_name().clone());

    let packages = app_data
        .config
        .packages
        .iter()
        .filter_map(|package_config| {
            let manifest = app_data.cache.update(package_config).ok()?;

            let name = package_config.get_public_name();
            let clear_name = filter_string(&package_config.key());

            Some(TemplatePackage {
                name: name.clone(),
                clear_name,
                versions: transform_version_info_for_templates(
                    &app_data.config,
                    Arc::clone(&manifest.manifest_for_templates),
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

    let body = app_data
        .handlebars
        .render("index", &data)
        .map_err(|err| ErrorInternalServerError(err))?;

    Ok(HttpResponse::Ok().body(body))
}
