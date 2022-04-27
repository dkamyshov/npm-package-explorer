use std::sync::Arc;

use crate::{common::AppData, error::NpmPackageServerError};
use rouille::{Response, ResponseBody};
use serde::Serialize;

pub fn list_versions_handler(
    app_data: Arc<AppData>,
    jsonp: Option<String>,
) -> Result<Response, NpmPackageServerError> {
    #[derive(Serialize)]
    struct VersionsListItem {
        name: String,
        versions: Vec<String>,
    }

    let result: Vec<VersionsListItem> = app_data
        .config
        .packages
        .iter()
        .filter_map(|package_config| {
            let manifest = app_data
                .manifest_repository
                .get_manifest(package_config)
                .ok()?;

            Some(VersionsListItem {
                name: package_config.get_public_name().clone(),
                versions: manifest
                    .versions
                    .iter()
                    .map(|version| version.version.to_string())
                    .collect(),
            })
        })
        .collect();

    if let Some(callback_name) = jsonp {
        let serialized_content = serde_json::to_string(&result)?;

        Ok(Response {
            status_code: 200,
            headers: vec![("Content-Type".into(), "text/html; charset=utf-8".into())],
            data: ResponseBody::from_string(format!("{}({})", callback_name, serialized_content)),
            upgrade: None,
        })
    } else {
        Ok(Response::json(&result))
    }
}
