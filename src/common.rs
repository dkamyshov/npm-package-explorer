use crate::{config::Config, npm_registry::CachedManifestRepository};
use handlebars::Handlebars;

pub struct AppData<'a> {
    pub config: Config,
    pub cache: CachedManifestRepository,
    pub handlebars: Handlebars<'a>,
}
