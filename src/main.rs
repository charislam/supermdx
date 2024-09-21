use dashmap::DashMap;
use markdown::{mdast::Node, to_mdast};
use tower_lsp::{jsonrpc, lsp_types::*, Client, LanguageServer, LspService, Server};

mod ast;
mod config;
mod nodes;
mod parser;

use crate::ast::{find_deepest_match, get_ancestor_chain};
use crate::config::Config;
use crate::nodes::NodeExt;
use crate::parser::get_parser_options;

#[derive(Debug)]
pub struct Backend {
    client: Client,
    config: Config,
    ast_map: DashMap<String, Node>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        self.initialize_config(&params).await;

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

        if let Some(Node::MdxJsxFlowElement(element)) =
            find_deepest_match(&ancestor_chain, |node| node.is_partial())
        {
            let uri = self.config.find_matching_partial(element);

            return Ok(
                // We just want to go to the file, it doesn't matter where.
                uri.map(|uri| GotoDefinitionResponse::Scalar(Location::new(uri, Range::default()))),
            );
        }

        Ok(None)
    }
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            ast_map: DashMap::new(),
            config: Default::default(),
        }
    }

    async fn on_change(&self, uri: &Url, text: &str) {
        let ast = to_mdast(text, &get_parser_options());
        if ast.is_ok() {
            let ast = ast.unwrap();
            self.ast_map.insert(uri.to_string(), ast);
        }
    }

    async fn initialize_config(&self, params: &InitializeParams) {
        let _ = self.config.0.lock().unwrap().update(params);
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::build(|client| Backend::new(client)).finish();

    Server::new(stdin, stdout, socket).serve(service).await;
}
