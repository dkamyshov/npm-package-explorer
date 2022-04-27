use crate::{
    config::Config,
    npm_registry::{DownloadManager, ManifestRepository},
};
use handlebars::Handlebars;
use regex::Regex;
use urlencoding::encode;

pub struct AppData<'a> {
    pub config: Config,
    pub manifest_repository: ManifestRepository,
    pub download_manager: DownloadManager,
    pub handlebars: Handlebars<'a>,
}

pub fn filter_string(source: &str) -> String {
    let re: Regex = Regex::new(r"[^a-zA-Z0-9]").unwrap();

    re.replace_all(encode(source).as_ref(), "_").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filters_characters_from_url() {
        assert_eq!(
            filter_string("https://registry.npmjs.com/@scope/name"),
            "https_3A_2F_2Fregistry_npmjs_com_2F_40scope_2Fname"
        );
    }
}
