use markdown::mdast::{AttributeContent, AttributeValue, MdxJsxFlowElement};
use tower_lsp::lsp_types::Url;

use crate::config::Config;

impl Config {
    pub fn find_matching_partial(&self, node: &MdxJsxFlowElement) -> Option<Url> {
        let config = self.0.lock().unwrap();
        let partials_dirs = &config.partials_dirs;
        let workspace_root = &config.workspace_root;

        if partials_dirs.is_empty() || workspace_root.is_none() {
            return None;
        }

        let src = node.attributes.iter().find_map(|attr| match attr {
            AttributeContent::Property(property) if property.name == "src" => {
                property.value.as_ref().and_then(|value| match value {
                    AttributeValue::Literal(string) => Some(string.clone()),
                    _ => return None,
                })
            }
            _ => None,
        })?;

        // We could probably fetch all the partials in a single pass, then
        // update on directory change, but that's a bit more complex. For now,
        // we'll fetch the partials on demand.
        for dir in partials_dirs {
            let dir_path = workspace_root.clone().unwrap().join(dir);
            if dir_path.exists() {
                let partial_path = dir_path.clone().join(&src);
                if partial_path.exists() {
                    return Some(Url::from_file_path(partial_path).unwrap());
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    use log::debug;
    use markdown::to_mdast;
    use tempfile::TempDir;

    use crate::config::{Config, ConfigValues};
    use crate::parser::get_parser_options;

    use super::*;

    #[test]
    fn test_find_matching_partial() {
        let ast = to_mdast(
            r#"
# Hello

<$Partial src="hello.mdx" />
"#,
            &get_parser_options(),
        )
        .unwrap();
        debug!("{:#?}", ast);
    }

    fn create_config(workspace_root: PathBuf, partials_dirs: Vec<PathBuf>) -> Config {
        let config = Config(Arc::new(Mutex::new(ConfigValues {
            workspace_root: Some(workspace_root),
            partials_dirs,
        })));
        config
    }

    fn create_partial(dir: &TempDir, name: &str, content: &str) -> PathBuf {
        let path = dir.path().join(name);
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_find_matching_partial_exists() {
        let workspace = TempDir::new().unwrap();
        let partials_dir = workspace.path().join("partials");
        fs::create_dir(&partials_dir).unwrap();

        create_partial(&workspace, "partials/hello.mdx", "# Hello");

        let backend = create_config(
            workspace.path().to_path_buf(),
            vec![PathBuf::from("partials")],
        );

        let ast = to_mdast(r#"<$Partial src="hello.mdx" />"#, &get_parser_options()).unwrap();
        let partial = ast.children().unwrap().get(0).unwrap();

        if let markdown::mdast::Node::MdxJsxFlowElement(element) = partial {
            let result = backend.find_matching_partial(&element);
            assert!(result.is_some());
            assert_eq!(
                result.unwrap(),
                Url::from_file_path(workspace.path().join("partials/hello.mdx")).unwrap()
            );
        } else {
            panic!("Expected MdxJsxFlowElement");
        }
    }

    #[test]
    fn test_find_matching_partial_not_exists() {
        let workspace = TempDir::new().unwrap();
        let partials_dir = workspace.path().join("partials");
        fs::create_dir(&partials_dir).unwrap();

        let backend = create_config(
            workspace.path().to_path_buf(),
            vec![PathBuf::from("partials")],
        );

        let ast = to_mdast(
            r#"<$Partial src="nonexistent.mdx" />"#,
            &get_parser_options(),
        )
        .unwrap();
        let partial = ast.children().unwrap().get(0).unwrap();

        if let markdown::mdast::Node::MdxJsxFlowElement(element) = partial {
            let result = backend.find_matching_partial(&element);
            assert!(result.is_none());
        } else {
            panic!("Expected MdxJsxFlowElement");
        }
    }

    #[test]
    fn test_find_matching_partial_multiple_dirs() {
        let workspace = TempDir::new().unwrap();
        fs::create_dir(workspace.path().join("partials1")).unwrap();
        fs::create_dir(workspace.path().join("partials2")).unwrap();

        create_partial(&workspace, "partials2/hello.mdx", "# Hello");

        let backend = create_config(
            workspace.path().to_path_buf(),
            vec![PathBuf::from("partials1"), PathBuf::from("partials2")],
        );

        let ast = to_mdast(r#"<$Partial src="hello.mdx" />"#, &get_parser_options()).unwrap();
        let partial = ast.children().unwrap().get(0).unwrap();

        if let markdown::mdast::Node::MdxJsxFlowElement(element) = partial {
            let result = backend.find_matching_partial(&element);
            assert!(result.is_some());
            assert_eq!(
                result.unwrap(),
                Url::from_file_path(workspace.path().join("partials2/hello.mdx")).unwrap()
            );
        } else {
            panic!("Expected MdxJsxFlowElement");
        }
    }
}
