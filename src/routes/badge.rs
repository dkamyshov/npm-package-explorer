use crate::{
    common::AppData,
    error::{NpmPackageServerError, PackageTrackingError},
};
use badgen::{badge, Color, Style};
use rouille::{Response, ResponseBody};
use std::sync::Arc;

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

fn error_badge_response(description: &str, status_code: u16) -> Response {
    let badge_content =
        render_badge("error", description, Color::Red).unwrap_or(String::from(FATAL_ERROR_BADGE));

    Response {
        status_code,
        headers: vec![("Content-Type".into(), "image/svg+xml; charset=utf-8".into())],
        data: ResponseBody::from_string(badge_content),
        upgrade: None,
    }
}

fn render_badge(
    title: &str,
    description: &str,
    color: Color<'_>,
) -> Result<String, NpmPackageServerError> {
    let mut style = Style::classic();
    style.background = color;

    badge(&style, description, Some(title))
        .map_err(|error| NpmPackageServerError::BadgeRendering(error))
}

fn badge_handler_inner(
    app_data: Arc<AppData>,
    package_name: Option<String>,
) -> Result<Response, NpmPackageServerError> {
    let package_name = package_name
        .clone()
        .ok_or(NpmPackageServerError::PackageNameIsNotSpecified)?;

    let package_config = app_data.config.get_package(&package_name).ok_or(
        PackageTrackingError::PackageIsNotTracked(package_name.to_string()),
    )?;

    let manifest = app_data.manifest_repository.get_manifest(package_config)?;

    let version = manifest
        .versions
        .get(0)
        .ok_or(PackageTrackingError::NoVersions(package_name.to_string()))?;

    let label = version.version.to_string();

    let body = render_badge("npm", &label, Color::Green)?;

    Ok(Response {
        status_code: 200,
        headers: vec![
            ("Content-Type".into(), "image/svg+xml; charset=utf-8".into()),
            ("cache-control".into(), "public, max-age=900".into()),
        ],
        data: ResponseBody::from_string(body),
        upgrade: None,
    })
}

pub fn badge_handler(
    app_data: Arc<AppData>,
    package_name: Option<String>,
) -> Result<Response, NpmPackageServerError> {
    let result = badge_handler_inner(app_data, package_name);

    match result {
        Ok(response) => Ok(response),
        Err(error) => Ok(match error {
            NpmPackageServerError::PackageNameIsNotSpecified => {
                error_badge_response("bad request", 400)
            }
            NpmPackageServerError::BadgeRendering(_) => {
                error_badge_response("failed to render badge", 500)
            }
            NpmPackageServerError::ManifestFetchError(_) => {
                error_badge_response("couldn't fetch manifest", 500)
            }
            NpmPackageServerError::PackageTrackingError(_) => {
                error_badge_response("package is not tracked or there are no versions", 404)
            }
            _ => error_badge_response("internal server error", 500),
        }),
    }
}
