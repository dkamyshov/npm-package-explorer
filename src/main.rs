use crate::common::AppData;
use crate::config::Config;
use crate::npm_registry::CachedManifestRepository;
use crate::routes::{badge_handler, index_handler, list_versions_handler, show_handler};
use actix_files::Files;
use actix_web::{self, middleware::Logger, web::Data, App, HttpServer};
use handlebars::Handlebars;

mod common;
mod config;
mod error;
mod npm_registry;
mod request;
mod routes;

#[macro_use]
extern crate serde_json;

extern crate derive_more;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::builder().format_timestamp_millis().init();

    let app_data = Data::new(AppData {
        config: Config::from_file("./npm-package-explorer.config.toml").unwrap(),
        cache: CachedManifestRepository::new(),
        handlebars: {
            let mut handlebars = Handlebars::new();
            handlebars
                .register_templates_directory(".html", "./static/templates")
                .unwrap();
            handlebars
        },
    });

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(Data::clone(&app_data))
            .service(show_handler)
            .service(badge_handler)
            .service(index_handler)
            .service(list_versions_handler)
            .service(Files::new("/static", "./static/files"))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
