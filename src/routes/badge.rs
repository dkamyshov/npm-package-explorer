use crate::common::AppData;
use actix_web::http::header;
use actix_web::web::{Data, Query};
use actix_web::{error::ErrorInternalServerError, ResponseError};
use actix_web::{get, http::StatusCode};
use actix_web::{HttpResponse, Result as ActixResult};
use badgen::{badge, Color, Style};
use derive_more::Display;
use serde_derive::Deserialize;

#[derive(Deserialize, Debug)]
pub struct QueryParams {
    package: Option<String>,
}

#[derive(Display, Debug)]
pub enum BadgeError {
    InvalidPackageName,
    PackageIsNotTracked,
    InvalidVersion,
    CacheUpdateFail,
    InternalServerError,
}

impl ResponseError for BadgeError {
    fn status_code(&self) -> StatusCode {
        match *self {
            BadgeError::InvalidPackageName => StatusCode::BAD_REQUEST,
            BadgeError::PackageIsNotTracked => StatusCode::NOT_FOUND,
            BadgeError::InvalidVersion => StatusCode::NOT_FOUND,
            BadgeError::CacheUpdateFail => StatusCode::INTERNAL_SERVER_ERROR,
            BadgeError::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let description = match *self {
            BadgeError::InvalidPackageName => "bad request",
            BadgeError::PackageIsNotTracked => "package is not tracked",
            BadgeError::CacheUpdateFail => "cache update failed",
            BadgeError::InvalidVersion => "version is not found",
            BadgeError::InternalServerError => "internal server error",
        };

        let badge_content = render_badge("error", description, Color::Red).unwrap();

        HttpResponse::Ok()
            .content_type("image/svg+xml")
            .body(badge_content)
    }
}

fn render_badge(title: &str, description: &str, color: Color<'_>) -> ActixResult<String> {
    let mut style = Style::classic();
    style.background = color;

    badge(&style, description, Some(title))
        .map_err(|_| ErrorInternalServerError("failed to render badge"))
}

#[get("/badge")]
pub async fn badge_handler(
    query: Query<QueryParams>,
    app_data: Data<AppData<'_>>,
) -> ActixResult<HttpResponse, BadgeError> {
    let package_name = query
        .package
        .clone()
        .ok_or(BadgeError::InvalidPackageName)?;

    let package_config = app_data
        .config
        .get_package(&package_name)
        .ok_or(BadgeError::PackageIsNotTracked)?;

    let manifest = app_data
        .cache
        .update(package_config)
        .map_err(|_| BadgeError::CacheUpdateFail)?;

    let version = manifest
        .manifest_for_templates
        .get(0)
        .ok_or(BadgeError::InvalidVersion)?;

    let label = version.version.clone();

    let body =
        render_badge("npm", &label, Color::Green).map_err(|_| BadgeError::InternalServerError)?;

    Ok(HttpResponse::Ok()
        .content_type("image/svg+xml")
        .set_header(header::CACHE_CONTROL, "public, max-age=900")
        .body(body))
}
