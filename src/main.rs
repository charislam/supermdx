use dashmap::DashMap;
use markdown::{mdast::Node, to_mdast};
use tokio::io::{self, AsyncBufReadExt, BufReader};
use tokio_stream::StreamExt;
use tokio_util::bytes::Bytes;
use tokio_util::io::StreamReader;
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

#[cfg(debug_assertions)]
#[tokio::main]
async fn main() {
    env_logger::init();

    let (stdin_tx, stdin_rx) = tokio::sync::mpsc::channel(100);

    tokio::spawn(async move {
        let stdin = io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        loop {
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    println!("Stdin closed, but keeping server alive.");
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    continue;
                }
                Ok(_) => {
                    println!("Received: {:?}", line.trim());
                    if let Err(e) = stdin_tx.send(line.clone()).await {
                        eprintln!("Failed to send line to channel: {}", e);
                    }
                    line.clear(); // Clear the line for the next read
                }
                Err(e) => {
                    eprintln!("Error reading from stdin: {}", e);
                }
            }
        }
    });

    let stdin = StreamReader::new(
        tokio_stream::wrappers::ReceiverStream::new(stdin_rx)
            .map(|line| Ok::<_, io::Error>(Bytes::from(line))),
    );
    let stdout = io::stdout();

    let (service, socket) = LspService::new(|client| Backend::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}

#[cfg(not(debug_assertions))]
#[tokio::main]
async fn main() {
    env_logger::init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}
