use crate::lsp::json_rpc::{JsonRpc, JsonRpcMessage};
use crate::lsp::process::Process;
use crate::lsp::{JsonRpcHandler, ProcessHandler};
use crate::utils::file_utils::search_directories;
use async_trait::async_trait;
use log::{debug, error, warn};
use lsp_types::{
    ClientCapabilities, DidOpenTextDocumentParams, DocumentSymbolClientCapabilities,
    DocumentSymbolParams, DocumentSymbolResponse, GotoDefinitionParams, GotoDefinitionResponse,
    InitializeParams, InitializeResult, Location, PartialResultParams, Position, ReferenceContext,
    ReferenceParams, TextDocumentClientCapabilities, TextDocumentIdentifier,
    TextDocumentPositionParams, Url, WorkDoneProgressParams, WorkspaceFolder,
    WorkspaceSymbolParams, WorkspaceSymbolResponse,
};
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::oneshot::{channel, Sender};
use tokio::sync::Mutex;

use crate::utils::workspace_documents::{WorkspaceDocumentsHandler, DEFAULT_EXCLUDE_PATTERNS};

#[derive(Clone)]
pub struct PendingRequests {
    channels: Arc<Mutex<HashMap<u64, Sender<JsonRpcMessage>>>>,
}

impl PendingRequests {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn add(
        &self,
        id: u64,
        sender: Sender<JsonRpcMessage>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.channels.lock().await.insert(id, sender);
        Ok(())
    }

    async fn remove(
        &self,
        id: u64,
    ) -> Result<Option<Sender<JsonRpcMessage>>, Box<dyn Error + Send + Sync>> {
        Ok(self.channels.lock().await.remove(&id))
    }
}

#[async_trait]
pub trait LspClient: Send {
    async fn initialize(
        &mut self,
        root_path: String,
    ) -> Result<InitializeResult, Box<dyn Error + Send + Sync>> {
        debug!("Initializing LSP client with root path: {:?}", root_path);
        self.start_response_listener().await?;
        let workspace_folders = self.find_workspace_folders(root_path.clone()).await?;
        let params = self.get_initialize_params(workspace_folders);

        let result = self
            .send_request("initialize", Some(serde_json::to_value(params)?))
            .await?;
        let init_result: InitializeResult = serde_json::from_value(result)?;
        debug!("Initialization successful: {:?}", init_result);
        self.send_initialized().await?;
        Ok(init_result)
    }

    async fn send_request(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn Error + Send + Sync>> {
        let (id, request) = self.get_json_rpc().create_request(method, params);

        let (response_sender, response_receiver) = channel::<JsonRpcMessage>();
        self.get_pending_requests().add(id, response_sender).await?;

        let message = format!("Content-Length: {}\r\n\r\n{}", request.len(), request);
        self.get_process().send(&message).await?;

        let response = response_receiver
            .await
            .map_err(|e| format!("Failed to receive response: {}", e))?;

        if let Some(result) = response.result {
            Ok(result)
        } else if let Some(error) = response.error {
            Err(error.into())
        } else {
            Ok(serde_json::Value::Null)
        }
    }

    async fn start_response_listener(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let process = self.get_process().clone();
        let pending_requests = self.get_pending_requests().clone();
        let json_rpc = self.get_json_rpc().clone();

        tokio::spawn(async move {
            loop {
                match process.receive().await {
                    Ok(raw_response) => {
                        match json_rpc.parse_message(&raw_response) {
                            Ok(message) => {
                                debug!("Parsed message id: {:?}", message.id);
                                if let Some(id) = message.id {
                                    debug!("Received response for request {}", id);
                                    if let Some(sender) = pending_requests.remove(id).await.unwrap()
                                    {
                                        let message_clone = message.clone();
                                        if sender.send(message_clone).is_err() {
                                            error!("Failed to send response for request {}", id);
                                        }
                                    } else {
                                        error!("Failed to remove pending request for {}", id);
                                        error!("Message: {:?}", message);
                                    }
                                } else {
                                    // Handle notifications or other non-request messages
                                    debug!("Received non-request message: {:?}", message);
                                }
                            }
                            Err(e) => error!("Failed to parse message: {}", e),
                        }
                    }
                    Err(e) => error!("Error receiving message: {}", e),
                }
            }
        });

        Ok(())
    }

    async fn send_initialized(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        debug!("Sending 'initialized' notification");
        let notification = self
            .get_json_rpc()
            .create_notification("initialized", serde_json::json!({}));
        let message = format!(
            "Content-Length: {}\r\n\r\n{}",
            notification.len(),
            notification
        );
        self.get_process().send(&message).await
    }

    async fn text_document_did_open(
        &mut self,
        item: lsp_types::TextDocumentItem,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        debug!("Sending 'didOpen' notification for document: {}", item.uri);
        let params = DidOpenTextDocumentParams {
            text_document: item,
        };
        let notification = self
            .get_json_rpc()
            .create_notification("textDocument/didOpen", serde_json::to_value(params)?);
        let message = format!(
            "Content-Length: {}\r\n\r\n{}",
            notification.len(),
            notification
        );
        self.get_process().send(&message).await
    }

    async fn text_document_definition(
        &mut self,
        file_path: &str,
        position: Position,
    ) -> Result<GotoDefinitionResponse, Box<dyn Error + Send + Sync>> {
        debug!(
            "Requesting goto definition for {}, line {}, character {}",
            file_path, position.line, position.character
        );
        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::from_file_path(file_path).unwrap(),
                },
                position: position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let result = self
            .send_request(
                "textDocument/definition",
                Some(serde_json::to_value(params)?),
            )
            .await?;
        let goto_resp: GotoDefinitionResponse = serde_json::from_value(result)?;
        debug!("Received goto definition response");
        Ok(goto_resp)
    }

    // TODO re-implement using textDocument/symbol
    #[allow(unused)]
    async fn workspace_symbols(
        &mut self,
        query: &str,
    ) -> Result<WorkspaceSymbolResponse, Box<dyn Error + Send + Sync>> {
        debug!("Requesting workspace symbols with query: {}", query);
        let params = WorkspaceSymbolParams {
            query: query.to_string(),
            ..Default::default()
        };
        let (id, request) = self
            .get_json_rpc()
            .create_request("workspace/symbol", Some(serde_json::to_value(params)?));
        let message = format!("Content-Length: {}\r\n\r\n{}", request.len(), request);
        self.get_process().send(&message).await?;

        let response = self
            .receive_response()
            .await?
            .ok_or("No response received")?;
        if let Some(result) = response.result {
            let symbols: WorkspaceSymbolResponse = serde_json::from_value(result)?;
            Ok(symbols)
        } else if let Some(error) = response.error {
            error!("Workspace symbols error: {:?}", error);
            Err(error.into())
        } else {
            Err("Unexpected workspace symbols response".into())
        }
    }

    #[allow(unused)]
    #[deprecated(note = "Use ast grep instead")]
    async fn text_document_symbols(
        &mut self,
        file_path: &str,
    ) -> Result<DocumentSymbolResponse, Box<dyn Error + Send + Sync>> {
        debug!("Requesting document symbols for {}", file_path);
        let params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier {
                uri: Url::from_file_path(file_path).unwrap(),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let result = self
            .send_request(
                "textDocument/documentSymbol",
                Some(serde_json::to_value(params)?),
            )
            .await?;

        let symbols: DocumentSymbolResponse = serde_json::from_value(result)?;
        debug!("Received document symbols response");
        Ok(symbols)
    }

    async fn text_document_reference(
        &mut self,
        file_path: &str,
        position: Position,
        include_declaration: bool,
    ) -> Result<Vec<Location>, Box<dyn Error + Send + Sync>> {
        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::from_file_path(file_path).map_err(|_| "Invalid file path")?,
                },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: ReferenceContext {
                include_declaration,
            },
        };

        let result = self
            .send_request(
                "textDocument/references",
                Some(serde_json::to_value(params)?),
            )
            .await?;

        let references: Vec<Location> = serde_json::from_value(result)?;
        debug!("Received references response");
        Ok(references)
    }

    async fn receive_response(
        &mut self,
    ) -> Result<Option<JsonRpcMessage>, Box<dyn Error + Send + Sync>> {
        debug!("Awaiting response from LSP server");
        // TODO this could be an inf loop, though timeout in receive will break it
        loop {
            let raw_response = self.get_process().receive().await?;
            let message = self.get_json_rpc().parse_message(&raw_response)?;
            debug!("Received response: {:?}", message);

            if let Some(msg_type) = &message.method {
                if msg_type == "window/logMessage" {
                    debug!("Captured log message, continuing to next message");
                    continue;
                }
            }

            if message.id.is_some() {
                return Ok(Some(message));
            }
        }
    }

    fn get_process(&mut self) -> &mut ProcessHandler;

    fn get_json_rpc(&mut self) -> &mut JsonRpcHandler;

    fn get_root_files(&mut self) -> Vec<String> {
        vec![".git".to_string()]
    }

    fn get_pending_requests(&mut self) -> &mut PendingRequests;

    fn get_workspace_documents(&mut self) -> &mut WorkspaceDocumentsHandler;
    /// Sets up the workspace for the language server.
    ///
    /// Some language servers require specific commands to be run before
    /// workspace-wide features are available. For example:
    /// - TypeScript Language Server needs an explicit didOpen notification for each file
    /// - Rust Analyzer needs a reloadWorkspace command
    ///
    /// # Arguments
    ///
    /// * `root_path` - The root path of the workspace
    ///
    /// # Returns
    ///
    /// A Result containing () if successful, or a boxed Error if an error occurred
    #[allow(unused)]
    async fn setup_workspace(
        &mut self,
        root_path: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }

    async fn find_workspace_folders(
        &mut self,
        root_path: String,
    ) -> Result<Vec<WorkspaceFolder>, Box<dyn Error + Send + Sync>> {
        let mut workspace_folders: Vec<WorkspaceFolder> = Vec::new();
        let include_patterns = self
            .get_root_files()
            .into_iter()
            .map(|f| format!("**/{f}"))
            .collect();
        let exclude_patterns = DEFAULT_EXCLUDE_PATTERNS
            .iter()
            .map(|&s| s.to_string())
            .collect();

        match search_directories(&Path::new(&root_path), include_patterns, exclude_patterns) {
            Ok(dirs) => {
                for dir in dirs {
                    let folder_path = Path::new(&root_path).join(&dir);
                    if let Ok(uri) = Url::from_file_path(&folder_path) {
                        workspace_folders.push(WorkspaceFolder {
                            uri,
                            name: folder_path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("")
                                .to_string(),
                        });
                    }
                }
            }
            Err(e) => return Err(Box::new(e)),
        }

        if workspace_folders.is_empty() {
            // Fallback: use the root_path itself as a workspace folder
            warn!("No workspace folders found. Using root path as workspace.");
            if let Ok(uri) = Url::from_file_path(&root_path) {
                workspace_folders.push(WorkspaceFolder {
                    uri,
                    name: root_path.to_string(),
                });
            }
        }

        Ok(workspace_folders.into_iter().collect())
    }

    fn get_initialize_params(
        &mut self,
        workspace_folders: Vec<WorkspaceFolder>,
    ) -> InitializeParams {
        let mut capabilities = ClientCapabilities::default();
        capabilities.text_document = Some(TextDocumentClientCapabilities {
            document_symbol: Some(DocumentSymbolClientCapabilities {
                hierarchical_document_symbol_support: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        });

        capabilities.experimental = Some(serde_json::json!({
            "serverStatusNotification": true
        }));

        let params = InitializeParams {
            capabilities,
            workspace_folders: Some(workspace_folders.clone()),
            root_uri: Some(workspace_folders[0].uri.clone()),
            ..Default::default()
        };
        params
    }
}
