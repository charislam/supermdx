use std::path::PathBuf;

use serde_json::Value;
use tower_lsp::lsp_types::InitializeParams;

#[derive(Debug, Default, Clone)]
pub struct Config {
    pub partials_dirs: Vec<PathBuf>,
}

impl Config {
    pub fn update(&mut self, params: &InitializeParams) {
        if let Some(partials_dir) = params
            .initialization_options
            .as_ref()
            .and_then(|o| o.get("partials_dir"))
        {
            let dirs = match partials_dir {
                Value::String(s) => vec![PathBuf::from(s)],
                Value::Array(a) => a
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| PathBuf::from(s)))
                    .collect(),
                _ => vec![],
            };

            if let Some(root_path) = params.root_uri.as_ref().and_then(|p| p.to_file_path().ok()) {
                self.partials_dirs = dirs.into_iter().map(|dir| root_path.join(dir)).collect();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tempfile::TempDir;
    use tower_lsp::lsp_types::{InitializeParams, Url};

    use super::*;

    fn create_params(root_dir: &TempDir, partials_dir: Value) -> InitializeParams {
        let mut params = InitializeParams::default();
        params.root_uri = Some(Url::from_file_path(root_dir.path()).unwrap());
        params.initialization_options = Some(json!({
            "partials_dir": partials_dir
        }));
        params
    }

    #[test]
    fn test_update_with_single_partials_dir() {
        let root_dir = TempDir::new().unwrap();
        let params = create_params(&root_dir, json!("custom_partials"));

        let mut config = Config::default();
        config.update(&params);

        let expected_path = root_dir.path().join("custom_partials");
        assert_eq!(config.partials_dirs, vec![expected_path]);
    }

    #[test]
    fn test_update_with_multiple_partials_dirs() {
        let root_dir = TempDir::new().unwrap();
        let params = create_params(&root_dir, json!(["components/partials", "layouts"]));

        let mut config = Config::default();
        config.update(&params);

        let expected_paths = vec![
            root_dir.path().join("components").join("partials"),
            root_dir.path().join("layouts"),
        ];
        assert_eq!(config.partials_dirs, expected_paths);
    }

    #[test]
    fn test_update_without_partials_dir() {
        let root_dir = TempDir::new().unwrap();
        let mut params = InitializeParams::default();
        params.root_uri = Some(Url::from_file_path(root_dir.path()).unwrap());

        let mut config = Config::default();
        config.update(&params);

        assert!(config.partials_dirs.is_empty());
    }

    #[test]
    fn test_update_with_invalid_root_uri() {
        let mut params = InitializeParams::default();
        params.root_uri = Some(Url::parse("invalid://url").unwrap());
        params.initialization_options = Some(json!({
            "partials_dir": "custom_partials"
        }));

        let mut config = Config::default();
        config.update(&params);

        assert!(config.partials_dirs.is_empty());
    }

    #[test]
    fn test_update_with_non_string_partials_dir() {
        let root_dir = TempDir::new().unwrap();
        let params = create_params(&root_dir, json!(42));

        let mut config = Config::default();
        config.update(&params);

        assert!(config.partials_dirs.is_empty());
    }
}
