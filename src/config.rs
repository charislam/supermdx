use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use toml;
use tower_lsp::lsp_types::InitializeParams;

#[derive(Debug, Default, Clone)]
pub struct Config(pub Arc<Mutex<ConfigValues>>);

#[derive(Debug, Default, Clone, Deserialize)]
pub struct ConfigValues {
    pub partials_dirs: Vec<PathBuf>,
    pub workspace_root: Option<PathBuf>,
}

const CONFIG_FILE: &str = ".supermdx.toml";

impl ConfigValues {
    pub fn update(&mut self, params: &InitializeParams) -> Result<()> {
        self.workspace_root = params
            .root_uri
            .as_ref()
            .ok_or_else(|| anyhow!("No root URI provided"))?
            .to_file_path()
            .map_err(|_| anyhow!("Invalid root URI: unable to convert to file path"))?
            .into();

        let workspace_root = self.workspace_root.as_ref().unwrap();
        let config_path = workspace_root.join(CONFIG_FILE);

        if config_path.exists() {
            let config_str = fs::read_to_string(&config_path).with_context(|| {
                format!("Failed to read config file: {}", config_path.display())
            })?;

            let config: ConfigValues = toml::from_str(&config_str).with_context(|| {
                format!("Failed to parse config file: {}", config_path.display())
            })?;

            self.partials_dirs = config
                .partials_dirs
                .into_iter()
                .map(|dir| workspace_root.join(dir))
                .collect();

            Ok(())
        } else {
            Err(anyhow!("Config file not found: {}", config_path.display()))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::TempDir;
    use tower_lsp::lsp_types::Url;

    use super::*;

    fn setup_config_params(root_dir: &TempDir, content: &str) -> InitializeParams {
        let config_path = root_dir.path().join(CONFIG_FILE);
        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();

        let mut params = InitializeParams::default();
        params.root_uri = Some(Url::from_file_path(root_dir.path()).unwrap());
        params
    }

    #[test]
    fn test_update_with_single_partials_dir() {
        let root_dir = TempDir::new().unwrap();
        let params = setup_config_params(
            &root_dir,
            r#"
partials_dirs = ["custom_partials"]
"#,
        );

        let mut config_values = ConfigValues::default();
        config_values.update(&params).unwrap();

        let expected_path = root_dir.path().join("custom_partials");
        assert_eq!(
            config_values.workspace_root,
            Some(root_dir.path().to_path_buf())
        );
        assert_eq!(config_values.partials_dirs, vec![expected_path]);
    }

    #[test]
    fn test_update_with_multiple_partials_dirs() {
        let root_dir = TempDir::new().unwrap();
        let params = setup_config_params(
            &root_dir,
            r#"
partials_dirs = ["components/partials", "layouts"]
"#,
        );

        let mut config_values = ConfigValues::default();
        config_values.update(&params).unwrap();

        let expected_paths = vec![
            root_dir.path().join("components").join("partials"),
            root_dir.path().join("layouts"),
        ];
        assert_eq!(config_values.partials_dirs, expected_paths);
    }

    #[test]
    fn test_update_without_partials_dir() {
        let root_dir = TempDir::new().unwrap();
        let params = setup_config_params(
            &root_dir,
            r#"
# Empty config
"#,
        );

        let mut config_values = ConfigValues::default();
        let result = config_values.update(&params);

        assert!(result.is_err());
        assert!(config_values.partials_dirs.is_empty());
    }

    #[test]
    fn test_update_with_invalid_config() {
        let root_dir = TempDir::new().unwrap();
        let params = setup_config_params(
            &root_dir,
            r#"
partials_dirs = 42  # Invalid type
"#,
        );

        let mut config_values = ConfigValues::default();
        let result = config_values.update(&params);

        assert!(result.is_err());
        assert!(config_values.partials_dirs.is_empty());
    }

    #[test]
    fn test_update_with_nonexistent_file() {
        let root_dir = TempDir::new().unwrap();
        let mut params = InitializeParams::default();
        params.root_uri = Some(Url::from_file_path(root_dir.path()).unwrap());

        let mut config_values = ConfigValues::default();
        let result = config_values.update(&params);

        assert!(result.is_err());
        assert!(config_values.partials_dirs.is_empty());
    }

    #[test]
    fn test_update_without_root_uri() {
        let params = InitializeParams::default();

        let mut config_values = ConfigValues::default();
        let result = config_values.update(&params);

        assert!(result.is_err());
        assert!(config_values.workspace_root.is_none());
        assert!(config_values.partials_dirs.is_empty());
    }

    #[test]
    fn test_update_with_invalid_root_uri() {
        let mut params = InitializeParams::default();
        params.root_uri = Some(Url::parse("invalid://url").unwrap());

        let mut config_values = ConfigValues::default();
        let result = config_values.update(&params);

        assert!(result.is_err());
        assert!(config_values.workspace_root.is_none());
        assert!(config_values.partials_dirs.is_empty());
    }
}
