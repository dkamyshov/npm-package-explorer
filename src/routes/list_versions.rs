use crate::common::AppData;
use actix_web::{
    get,
    web::{Data, Query},
    HttpResponse, Result as ActixResult,
};
use serde::Serialize;
use serde_derive::Deserialize;

#[derive(Deserialize)]
pub struct QueryParams {
    jsonp: Option<String>,
}

#[get("/api/versions")]
pub async fn list_versions_handler(
    query: Query<QueryParams>,
    app_data: Data<AppData<'_>>,
) -> ActixResult<HttpResponse> {
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
            let manifest = app_data.cache.update(package_config).ok()?;

            Some(VersionsListItem {
                name: package_config.get_public_name().clone(),
                versions: manifest
                    .manifest_for_templates
                    .iter()
                    .map(|version| version.version.clone())
                    .collect(),
            })
        })
        .collect();

    let serialized_content = serde_json::to_string(&result)?;

    if let Some(callback_name) = &query.jsonp {
        Ok(HttpResponse::Ok()
            .content_type("application/javascript; charset=utf-8")
            .body(format!("{}({})", callback_name, serialized_content)))
    } else {
        Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(serialized_content))
    }
}
