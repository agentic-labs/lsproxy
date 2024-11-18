use crate::lsp::json_rpc::{JsonRpc, JsonRpcMessage};
use crate::lsp::process::Process;
use crate::lsp::{ExpectedMessageKey, InnerMessage, JsonRpcHandler, ProcessHandler};
use crate::utils::file_utils::search_directories;
use async_trait::async_trait;
use log::{debug, error, warn};
use lsp_types::{
    ClientCapabilities, DidOpenTextDocumentParams, DocumentSymbolClientCapabilities,
    GotoDefinitionParams, GotoDefinitionResponse, InitializeParams, InitializeResult, Location,
    PartialResultParams, Position, PublishDiagnosticsClientCapabilities, ReferenceContext,
    ReferenceParams, RenameParams, TagSupport, TextDocumentClientCapabilities,
    TextDocumentIdentifier, TextDocumentPositionParams, Url, WorkDoneProgressParams, WorkspaceEdit,
    WorkspaceFolder, WorkspaceSymbolParams, WorkspaceSymbolResponse,
};
use std::error::Error;
use std::path::Path;

use crate::utils::workspace_documents::{WorkspaceDocumentsHandler, DEFAULT_EXCLUDE_PATTERNS};

use super::PendingRequests;

#[async_trait]
pub trait LspClient: Send {
    async fn initialize(
        &mut self,
        root_path: String,
    ) -> Result<InitializeResult, Box<dyn Error + Send + Sync>> {
        debug!("Initializing LSP client with root path: {:?}", root_path);
        self.start_response_listener().await?;

        let params = self.get_initialize_params(root_path).await;

        let result = self
            .send_request("initialize", Some(serde_json::to_value(params)?))
            .await?;
        let init_result: InitializeResult = serde_json::from_value(result)?;
        debug!("Initialization successful: {:?}", init_result);
        self.send_initialized().await?;
        Ok(init_result)
    }

    fn get_capabilities(&mut self) -> ClientCapabilities {
        let mut capabilities = ClientCapabilities::default();
        capabilities.text_document = Some(TextDocumentClientCapabilities {
            document_symbol: Some(DocumentSymbolClientCapabilities {
                hierarchical_document_symbol_support: Some(true),
                ..Default::default()
            }),
            // Turn off diagnostics for performance, we don't use them at the moment
            publish_diagnostics: Some(PublishDiagnosticsClientCapabilities {
                related_information: Some(false),
                tag_support: Some(TagSupport { value_set: vec![] }),
                code_description_support: Some(false),
                data_support: Some(false),
                version_support: Some(false),
            }),
            ..Default::default()
        });

        capabilities.experimental = Some(serde_json::json!({
            "serverStatusNotification": true
        }));
        capabilities
    }

    async fn get_initialize_params(&mut self, root_path: String) -> InitializeParams {
        InitializeParams {
            capabilities: self.get_capabilities(),
            workspace_folders: Some(
                self.find_workspace_folders(root_path.clone())
                    .await
                    .unwrap(),
            ),
            root_uri: Some(Url::from_file_path(&root_path).unwrap()), // primarily for python
            ..Default::default()
        }
    }

    async fn send_request(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn Error + Send + Sync>> {
        let (id, request) = self.get_json_rpc().create_request(method, params);

        let mut response_receiver = self.get_pending_requests().add_request(id).await?;

        let message = format!("Content-Length: {}\r\n\r\n{}", request.len(), request);
        self.get_process().send(&message).await?;

        let response = response_receiver
            .recv()
            .await
            .map_err(|e| format!("Failed to receive response: {}", e))?;

        if let Some(result) = response.result {
            Ok(result)
        } else if let Some(error) = response.error.clone() {
            error!("Recieved error: {:?}", response);
            if error.message.starts_with("KeyError") {
                return Ok(serde_json::Value::Array(vec![]));
            }
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
                if let Ok(raw_response) = process.receive().await {
                    if let Ok(message) = json_rpc.parse_message(&raw_response) {
                        if let Some(id) = message.id {
                            debug!("Received response for request {}", id);
                            if let Ok(Some(sender)) = pending_requests.remove_request(id).await {
                                if sender.send(message.clone()).is_err() {
                                    error!("Failed to send response for request {}", id);
                                }
                            } else {
                                error!(
                                    "Failed to remove pending request {} - Message: {:?}",
                                    id, message
                                );
                            }
                        } else if let Some(params) = message
                            .params
                            .clone()
                            .and_then(|p| serde_json::from_value::<InnerMessage>(p).ok())
                        {
                            let message_key = ExpectedMessageKey {
                                method: message.method.clone().unwrap(),
                                message: params.message,
                            };
                            if let Some(sender) =
                                pending_requests.remove_notification(message_key).await
                            {
                                sender.send(message).unwrap();
                            }
                        }
                    }
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

        // If result is null, default to an empty array response instead of failing deserialization
        let goto_resp: GotoDefinitionResponse = if result.is_null() {
            GotoDefinitionResponse::Array(Vec::new())
        } else {
            serde_json::from_value(result)?
        };

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

    async fn text_document_reference(
        &mut self,
        file_path: &str,
        position: Position,
    ) -> Result<Vec<Location>, Box<dyn Error + Send + Sync>> {
        // TODO: the jedi language server doesn't appear to respect
        // The "includeDeclaration" param so we'll just say we're
        // always including it
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
                include_declaration: true,
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

    async fn text_document_rename(
        &mut self,
        file_path: &str,
        position: Position,
        new_name: String,
    ) -> Result<WorkspaceEdit, Box<dyn Error + Send + Sync>> {
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::from_file_path(file_path).unwrap_or_else(|_| {
                        panic!("Invalid file path: {}", file_path);
                    }),
                },
                position,
            },
            new_name,
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        let result = self
            .send_request("textDocument/rename", Some(serde_json::to_value(params)?))
            .await?;
        let workspace_edit: WorkspaceEdit = serde_json::from_value(result)?;
        Ok(workspace_edit)
    }
}
