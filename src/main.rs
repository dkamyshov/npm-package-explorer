use crate::common::AppData;
use crate::config::Config;
use crate::npm_registry::ManifestRepository;
use crate::routes::{badge_handler, index_handler, list_versions_handler};
use error::NpmPackageServerError;
use handlebars::Handlebars;
use npm_registry::DownloadManager;
use rouille::{match_assets, router, start_server, Request, Response};
use routes::show_handler;
use std::io;
use std::sync::Arc;

mod cache;
mod coalescer;
mod common;
mod config;
mod error;
mod npm_registry;
mod request;
mod routes;

#[macro_use]
extern crate serde_json;

fn result_to_response(result: Result<Response, NpmPackageServerError>) -> Response {
    match result {
        Ok(response) => response,
        Err(error) => {
            let message = error.to_string();

            match error {
                _ => Response::text(message).with_status_code(500),
            }
        }
    }
}

fn handler(request: &Request, app_data: Arc<AppData>) -> Response {
    {
        if let Some(nested_request) = request.remove_prefix("/static") {
            let assets_response = match_assets(&nested_request, "./static/files");

            if assets_response.is_success() {
                return assets_response;
            }
        }
    }

    router!(request,
        (GET) (/) => {
            let package_name = request.get_param("package");

            result_to_response(index_handler(
                Arc::clone(&app_data),
                package_name
            ))
        },
        (GET) (/api/versions) => {
            let jsonp = request.get_param("jsonp");

            result_to_response(list_versions_handler(
                Arc::clone(&app_data),
                jsonp
            ))
        },
        (GET) (/badge) => {
            let package_name = request.get_param("package");

            result_to_response(badge_handler(
                Arc::clone(&app_data),
                package_name
            ))
        },
        _ => {
            if let Some(nested_show_request) = request.remove_prefix("/show/") {
                let url = nested_show_request.url();

                return result_to_response(show_handler(
                    Arc::clone(&app_data),
                    url
                ));
            }

            rouille::Response::empty_404()
        }
    )
}

fn main() -> std::io::Result<()> {
    env_logger::builder().format_timestamp_millis().init();

    let app_data = Arc::new(AppData {
        config: Config::from_file("./npm-package-explorer.config.toml")
            .expect("config file ./npm-package-explorer.config.toml doesn't exist"),
        manifest_repository: ManifestRepository::new(),
        download_manager: DownloadManager::new(),
        handlebars: {
            let mut handlebars = Handlebars::new();
            handlebars
                .register_templates_directory(".html", "./static/templates")
                .unwrap();
            handlebars
        },
    });

    let listen_address = app_data.config.listen_address.clone();

    start_server(listen_address, move |request| {
        rouille::log(request, io::stdout(), || {
            handler(request, Arc::clone(&app_data))
        })
    });
}
