use std::str::FromStr;

use crate::error::NpmPackageServerError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageFileRequest {
    pub name: String,
    pub version: String,
    pub path: String,
}

impl FromStr for PackageFileRequest {
    type Err = NpmPackageServerError;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        let mut result = PackageFileRequest {
            name: String::new(),
            version: String::new(),
            path: String::new(),
        };

        let mut parts = s.split("/");

        match parts.next() {
            Some(s) => {
                result.name.push_str(s);

                if s.starts_with("@") {
                    match parts.next() {
                        Some(s) => {
                            result.name.push('/');
                            result.name.push_str(s);
                        }
                        None => {
                            return Err(NpmPackageServerError::PackageFileRequestParseError(
                                format!("invalid scoped name format: {}", s),
                            ));
                        }
                    }
                }
            }
            None => {
                return Err(NpmPackageServerError::PackageFileRequestParseError(
                    format!("invalid name format: {}", s),
                ));
            }
        }

        let version = parts.next();

        match version {
            Some(version) => {
                result.version.push_str(version);
            }
            None => {
                return Err(NpmPackageServerError::PackageFileRequestParseError(
                    format!("invalid name format (invalid or missing version): {}", s),
                ));
            }
        }

        if let Some(part) = parts.next() {
            result.path.push_str(part);
        }

        for part in parts {
            result.path.push('/');
            result.path.push_str(part);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regular_name() {
        let result = "react/0.1.0/README.md".parse::<PackageFileRequest>();

        assert_eq!(
            result.unwrap(),
            PackageFileRequest {
                name: "react".into(),
                version: "0.1.0".into(),
                path: "README.md".into()
            }
        );
    }

    #[test]
    fn test_regular_name_long_path() {
        let result = "react/0.1.0/lib/components/Component.js".parse::<PackageFileRequest>();

        assert_eq!(
            result.unwrap(),
            PackageFileRequest {
                name: "react".into(),
                version: "0.1.0".into(),
                path: "lib/components/Component.js".into()
            }
        );
    }

    #[test]
    fn test_regular_name_empty_path() {
        let result = "react/0.1.0/".parse::<PackageFileRequest>();

        assert_eq!(
            result.unwrap(),
            PackageFileRequest {
                name: "react".into(),
                version: "0.1.0".into(),
                path: "".into()
            }
        );
    }

    #[test]
    fn test_regular_name_no_trailing_slash() {
        let result = "react/0.1.0".parse::<PackageFileRequest>();

        assert_eq!(
            result.unwrap(),
            PackageFileRequest {
                name: "react".into(),
                version: "0.1.0".into(),
                path: "".into()
            }
        );
    }

    #[test]
    fn test_regular_name_invalid() {
        let result = "react".parse::<PackageFileRequest>();
        assert_eq!(result.is_err(), true);
    }

    #[test]
    fn test_scoped_name_empty_path() {
        let result = "@d11t/ui/0.1.0/".parse::<PackageFileRequest>();
        assert_eq!(
            result.unwrap(),
            PackageFileRequest {
                name: "@d11t/ui".into(),
                version: "0.1.0".into(),
                path: "".into()
            }
        );
    }
}
