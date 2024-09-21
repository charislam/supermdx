use markdown::mdast::Node;
use tower_lsp::lsp_types::Position;

mod partials;

const PARTIAL: &str = "$Partial";

pub trait NodeExt {
    fn contains_position(&self, position: &Position) -> bool;
    fn is_partial(&self) -> bool;
}

impl NodeExt for Node {
    fn contains_position(&self, position: &Position) -> bool {
        self.position()
            .map(|pos| {
                pos.start.line <= (position.line + 1) as usize
                    && pos.end.line >= (position.line + 1) as usize
                    && pos.start.column <= (position.character + 1) as usize
                    && pos.end.column >= (position.character + 1) as usize
            })
            .unwrap_or(false)
    }

    fn is_partial(&self) -> bool {
        match self {
            Node::MdxJsxFlowElement(element) => {
                element.name.as_ref().map_or(false, |name| name == PARTIAL)
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use log::debug;
    use markdown::to_mdast;

    use super::*;
    use crate::parser::get_parser_options;

    #[test]
    fn test_contains_position() {
        let ast = to_mdast(
            r#"
# Hello World

This is a test.
"#
            .trim(),
            &get_parser_options(),
        )
        .unwrap();
        debug!("{:#?}", ast);

        let position = Position {
            line: 0,
            character: 5,
        };

        let heading_node = ast.children().unwrap().get(0).unwrap();
        debug!("heading node: {:#?}", heading_node);
        assert!(heading_node.contains_position(&position));

        let paragraph_node = ast.children().unwrap().get(1).unwrap();
        debug!("paragraph node: {:#?}", paragraph_node);
        assert!(!paragraph_node.contains_position(&position));
    }

    #[test]
    fn test_partial() {
        let ast = to_mdast(
            r#"
# Hello

<$Partial />
"#
            .trim(),
            &get_parser_options(),
        )
        .unwrap();
        debug!("{:#?}", ast);

        let partial = ast.children().unwrap().get(1).unwrap();
        assert!(partial.is_partial());
    }

    #[test]
    fn test_partial_with_attributes() {
        let ast = to_mdast(
            r#"
# Hello

<$Partial foo="bar" />
"#
            .trim(),
            &get_parser_options(),
        )
        .unwrap();
        debug!("{:#?}", ast);

        let partial = ast.children().unwrap().get(1).unwrap();
        assert!(partial.is_partial());
    }
}
