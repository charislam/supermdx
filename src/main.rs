use ast::{find_deepest_match, get_ancestor_chain};
use dashmap::DashMap;
use markdown::{mdast::Node, to_mdast};
use parser::get_parser_options;
use tower_lsp::{jsonrpc, lsp_types::*, Client, LanguageServer};

mod ast;
mod nodes;
mod parser;

use crate::nodes::NodeExt;

#[derive(Debug)]
pub struct Backend {
    client: Client,
    ast_map: DashMap<String, Node>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                definition_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Server initialized!")
            .await
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "File opened!")
            .await;
        self.on_change(&params.text_document.uri, &params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "File changed!")
            .await;
        self.on_change(&params.text_document.uri, &params.content_changes[0].text)
            .await;
    }

    async fn did_save(&self, _: DidSaveTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "File saved!")
            .await;
    }

    async fn did_close(&self, _: DidCloseTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "File closed!")
            .await;
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> jsonrpc::Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let ast = self.ast_map.get(&uri.to_string()).unwrap();
        let ancestor_chain = get_ancestor_chain(&ast, &position);
        let deepest_match = find_deepest_match(&ancestor_chain, |node| node.is_partial());

        Ok(None)
    }
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            ast_map: DashMap::new(),
        }
    }

    async fn on_change(&self, uri: &Url, text: &str) {
        let ast = to_mdast(text, &get_parser_options());
        if ast.is_ok() {
            let ast = ast.unwrap();
            self.ast_map.insert(uri.to_string(), ast);
        }
    }
}

fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
use ctor::ctor;

#[cfg(test)]
#[ctor]
fn init_test_logger() {
    env_logger::init();
}
