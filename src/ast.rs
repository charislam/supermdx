use markdown::mdast::Node;
use tower_lsp::lsp_types::Position;

use crate::nodes::NodeExt;

pub fn get_ancestor_chain<'a>(ast: &'a Node, position: &Position) -> Vec<&'a Node> {
    let mut ancestor_chain = Vec::new();
    let mut current_node = Some(ast);

    if !ast.contains_position(&position) {
        return ancestor_chain;
    }

    while current_node.is_some() {
        let node = current_node.unwrap();
        ancestor_chain.push(node);
        let next = node.children().and_then(|children| {
            children
                .iter()
                .find(|child| child.contains_position(position))
        });
        current_node = next;
    }

    ancestor_chain
}

pub fn find_deepest_match<'a, F>(ancestor_chain: &'a Vec<&Node>, test: F) -> Option<&'a Node>
where
    F: Fn(&Node) -> bool,
{
    for ancestor in ancestor_chain.iter().rev() {
        if test(ancestor) {
            return Some(*ancestor);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use log::debug;
    use markdown::to_mdast;

    use super::*;
    use crate::parser::get_parser_options;

    #[test]
    fn test_get_ancestor_chain() {
        let ast = to_mdast(
            r#"
# Hello World

- Item 1
- Item 2
"#
            .trim(),
            &get_parser_options(),
        )
        .unwrap();
        debug!("{:#?}", ast);

        let position = Position {
            line: 2,
            character: 5,
        };

        let ancestor_chain = get_ancestor_chain(&ast, &position);
        debug!("ancestor chain: {:#?}", ancestor_chain);

        let list = ast.children().unwrap().get(1).unwrap();
        let list_item = list.children().unwrap().get(0).unwrap();
        let paragraph = list_item.children().unwrap().get(0).unwrap();
        let text = paragraph.children().unwrap().get(0).unwrap();
        let expected_ancestor_chain = vec![&ast, &list, &list_item, &paragraph, &text];

        assert_eq!(ancestor_chain, expected_ancestor_chain);
    }

    #[test]
    fn test_find_deepest_match() {
        let ast = to_mdast(
            r#"
# Hello World

- Item 1
- Item 2
  - Nested Item 1
"#
            .trim(),
            &get_parser_options(),
        )
        .unwrap();
        debug!("{:#?}", ast);

        let position = Position {
            line: 4,
            character: 5,
        };

        let ancestor_chain = get_ancestor_chain(&ast, &position);
        debug!("ancestor chain: {:#?}", ancestor_chain);

        let deepest_match = find_deepest_match(&ancestor_chain, |node| match node {
            Node::ListItem(_) => true,
            _ => false,
        });
        debug!("deepest match: {:#?}", deepest_match);

        let list = ast.children().unwrap().get(1).unwrap();
        let list_item = list.children().unwrap().get(1).unwrap();
        let nested_list = list_item.children().unwrap().get(1).unwrap();
        let nested_list_item = nested_list.children().unwrap().get(0).unwrap();

        assert_eq!(deepest_match, Some(nested_list_item));
    }
}
