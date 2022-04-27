use crate::{common::filter_string, error::NpmPackageServerError};
use serde_derive::{Deserialize, Serialize};
use std::{fs::read_to_string, path::Path};
use toml::from_str;

fn default_registry() -> String {
    String::from("https://registry.npmjs.org/")
}

fn default_ssl_verify() -> bool {
    true
}

fn default_index_file() -> String {
    String::from("index.html")
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Labels {
    pub title: String,
    pub banner: String,
    pub version: String,
    pub published: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PackageConfig {
    pub name: String,
    pub alias: Option<String>,
    #[serde(default = "default_registry")]
    pub registry: String,
    pub access_token: Option<String>,
    #[serde(default = "default_ssl_verify")]
    pub ssl_verify: bool,
    #[serde(default = "default_index_file")]
    pub index_file: String,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub listen_address: String,
    pub timeago_language: Option<String>,
    pub banner_gradient_left_color: String,
    pub banner_gradient_right_color: String,
    pub banner_color: String,
    pub labels: Labels,
    pub packages: Vec<PackageConfig>,
}

impl PackageConfig {
    pub fn get_public_name(&self) -> &String {
        if let Some(alias) = self.alias.as_ref() {
            return alias;
        }

        &self.name
    }

    pub fn key(&self) -> String {
        let mut result = String::new();

        result.push_str(&self.registry);
        result.push_str(&self.name);

        result
    }

    pub fn identifier_safe_key(&self) -> String {
        let mut result = String::from("explorer_");
        result.push_str(&self.key());
        filter_string(&result).to_lowercase()
    }
}

impl Config {
    pub fn get_package(&self, name: &str) -> Option<&PackageConfig> {
        for item in self.packages.iter() {
            let package_public_name = item.get_public_name();

            if package_public_name == name {
                return Some(item);
            }
        }

        None
    }

    pub fn get_first_package(&self) -> Option<&PackageConfig> {
        self.packages.get(0)
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Config, NpmPackageServerError> {
        Ok(from_str::<Config>(read_to_string(path)?.as_ref())?)
    }
}
