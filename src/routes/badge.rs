use crate::common::AppData;
use actix_web::http::header;
use actix_web::web::{Data, Query};
use actix_web::{error::ErrorInternalServerError, ResponseError};
use actix_web::{get, http::StatusCode};
use actix_web::{HttpResponse, Result as ActixResult};
use badgen::{badge, Color, Style};
use derive_more::Display;
use serde_derive::Deserialize;

const FATAL_ERROR_BADGE: &'static str = r###"
<svg
  width="159.6"
  height="20"
  viewBox="0 0 1596 200"
  xmlns="http://www.w3.org/2000/svg"
  role="img"
  aria-label="error: internal server error"
>
  <title>error: internal server error</title>
  <linearGradient id="a" x2="0" y2="100%">
    <stop offset="0" stop-opacity=".1" stop-color="#EEE" />
    <stop offset="1" stop-opacity=".1" />
  </linearGradient>
  <mask id="m"><rect width="1596" height="200" rx="30" fill="#FFF" /></mask>
  <g mask="url(#m)">
    <rect width="374" height="200" fill="#555" />
    <rect width="1222" height="200" fill="#E43" x="374" />
    <rect width="1596" height="200" fill="url(#a)" />
  </g>
  <g
    aria-hidden="true"
    fill="#fff"
    text-anchor="start"
    font-family="Verdana,DejaVu Sans,sans-serif"
    font-size="110"
  >
    <text x="60" y="148" textLength="274" fill="#000" opacity="0.25">
      error
    </text>
    <text x="50" y="138" textLength="274">error</text>
    <text x="429" y="148" textLength="1122" fill="#000" opacity="0.25">
      internal server error
    </text>
    <text x="419" y="138" textLength="1122">internal server error</text>
  </g>
</svg>

"###;

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

        let badge_content = render_badge("error", description, Color::Red)
            .unwrap_or(String::from(FATAL_ERROR_BADGE));

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
