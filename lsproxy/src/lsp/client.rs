use crate::lsp::json_rpc::JsonRpc;
use crate::lsp::process::Process;
use crate::lsp::{ExpectedMessageKey, JsonRpcHandler, ProcessHandler};
use crate::utils::file_utils::{detect_language_string, search_directories};
use lsp_types::{
    CallHierarchyItem, Location, Position,
    Range, Url,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tree_sitter::{Parser, Point, Query, QueryCursor, Tree};
use streaming_iterator::StreamingIterator;

// Types for manual call hierarchy implementation
#[derive(Debug, Clone)]
pub struct Package {
    pub path: String,
    // Add other package-related fields as needed
}

#[derive(Debug, Clone)]
pub struct Object {
    pub name: String,
    pub package_path: String,
    pub file_path: String, // Actual path to the file containing this object
    pub range: Range,
    pub node_range: (usize, usize), // start_byte, end_byte
    pub source: Arc<String>,
    pub tree: Arc<Tree>,
    pub is_reference: bool, // true if this is a reference (e.g. function call), false if it's a definition
}
use async_trait::async_trait;
use log::{debug, error, warn};
use lsp_types::{
    CallHierarchyPrepareParams, CallHierarchyClientCapabilities,
    CallHierarchyIncomingCall, CallHierarchyIncomingCallsParams,
    CallHierarchyOutgoingCall, CallHierarchyOutgoingCallsParams,
    ClientCapabilities, DidOpenTextDocumentParams, DocumentSymbolClientCapabilities,
    GotoDefinitionParams, GotoDefinitionResponse, InitializeParams, InitializeResult,
    PartialResultParams, PublishDiagnosticsClientCapabilities, ReferenceContext, ReferenceParams,
    TagSupport, TextDocumentClientCapabilities, TextDocumentIdentifier, TextDocumentItem,
    TextDocumentPositionParams, WorkDoneProgressParams, WorkspaceFolder,
};
use std::error::Error;

use crate::utils::workspace_documents::{
    DidOpenConfiguration, WorkspaceDocuments, WorkspaceDocumentsHandler, DEFAULT_EXCLUDE_PATTERNS,
};

use super::PendingRequests;

#[async_trait]
pub trait LspClient: Send {
    async fn initialize(
        &mut self,
        root_path: String,
    ) -> Result<InitializeResult, Box<dyn Error + Send + Sync>> {
        debug!("Initializing LSP client with root path: {:?}", root_path);
        self.start_response_listener().await?;

        let params = self.get_initialize_params(root_path).await?;

        let result = self
            .send_request("initialize", Some(serde_json::to_value(params)?))
            .await?;
        let init_result: InitializeResult = serde_json::from_value(result)?;
        debug!("Initialization successful. Server capabilities: {:#?}", init_result.capabilities);
        
        // Specifically log call hierarchy support
        if let Some(call_hierarchy) = init_result.capabilities.call_hierarchy_provider {
            debug!("Server supports call hierarchy: {:#?}", call_hierarchy);
        } else {
            debug!("Server does not advertise call hierarchy support");
        }
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
            call_hierarchy: Some(CallHierarchyClientCapabilities {
                dynamic_registration: Some(true),
            }),
            ..Default::default()
        });

        capabilities.experimental = Some(serde_json::json!({
            "serverStatusNotification": true
        }));
        capabilities
    }

    async fn get_initialize_params(
        &mut self,
        root_path: String,
    ) -> Result<InitializeParams, Box<dyn Error + Send + Sync>> {
        let workspace_folders = self.find_workspace_folders(root_path.clone()).await?;
        Ok(InitializeParams {
            capabilities: self.get_capabilities(),
            workspace_folders: Some(workspace_folders),
            root_uri: Some(Url::from_file_path(&root_path).unwrap()), // primarily for python
            ..Default::default()
        })
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
                        } else if let Some(params) = message.params.clone() {
                            let message_key = ExpectedMessageKey {
                                method: message.method.clone().unwrap(),
                                params: params,
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
            "text_document_definition: Starting for {}, line {}, character {}",
            file_path, position.line, position.character
        );

        let needs_open = {
            let workspace_documents = self.get_workspace_documents();
            workspace_documents.get_did_open_configuration() == DidOpenConfiguration::Lazy
                && !workspace_documents.is_did_open_document(file_path)
        };

        // If needed, read the document text and send didOpen
        if needs_open {
            let document_text = self
                .get_workspace_documents()
                .read_text_document(&PathBuf::from(file_path), None)
                .await?;

            self.text_document_did_open(TextDocumentItem {
                uri: Url::from_file_path(file_path).unwrap(),
                language_id: detect_language_string(file_path)?,
                version: 1,
                text: document_text,
            })
            .await?;

            self.get_workspace_documents()
                .add_did_open_document(file_path);
        }

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

        debug!(
            "text_document_definition: Sending request with params: {:?}",
            params
        );
        let result = self
            .send_request(
                "textDocument/definition",
                Some(serde_json::to_value(params)?),
            )
            .await?;
        debug!(
            "text_document_definition: Raw response: {:?}",
            result
        );

        // If result is null, default to an empty array response instead of failing deserialization
        let goto_resp: GotoDefinitionResponse = if result.is_null() {
            debug!("text_document_definition: Got null response");
            GotoDefinitionResponse::Array(Vec::new())
        } else {
            match serde_json::from_value::<GotoDefinitionResponse>(result.clone()) {
                Ok(resp) => {
                    debug!("text_document_definition: Successfully parsed response: {:?}", resp);
                    resp
                }
                Err(e) => {
                    debug!("text_document_definition: Failed to parse response: {}", e);
                    return Err(e.into());
                }
            }
        };

        debug!("Received goto definition response");
        Ok(goto_resp)
    }

    async fn text_document_reference(
        &mut self,
        file_path: &str,
        position: Position,
    ) -> Result<Vec<Location>, Box<dyn Error + Send + Sync>> {
        // Get the configuration and check if document is opened first
        let needs_open = {
            let workspace_documents = self.get_workspace_documents();
            workspace_documents.get_did_open_configuration() == DidOpenConfiguration::Lazy
                && !workspace_documents.is_did_open_document(file_path)
        };

        // If needed, read the document text and send didOpen
        if needs_open {
            let document_text = self
                .get_workspace_documents()
                .read_text_document(&PathBuf::from(file_path), None)
                .await?;

            self.text_document_did_open(TextDocumentItem {
                uri: Url::from_file_path(file_path).unwrap(),
                language_id: detect_language_string(file_path)?,
                version: 1,
                text: document_text,
            })
            .await?;

            self.get_workspace_documents()
                .add_did_open_document(file_path);
        }

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

        debug!(
            "Received response from LSP server for references at {}:{}",
            file_path, position.line
        );

        let references: Vec<Location> = if result.is_null() {
            debug!(
                "LSP server returned null for references at {}:{} - treating as empty result",
                file_path, position.line
            );
            Vec::new()
        } else {
            debug!("LSP server returned non-null response for references, attempting to parse");
            match serde_json::from_value::<Vec<Location>>(result.clone()) {
                Ok(locs) => {
                    debug!(
                        "Successfully parsed {} reference locations from LSP response",
                        locs.len()
                    );
                    locs
                }
                Err(e) => {
                    error!(
                        "Failed to parse LSP response for references at {}:{}: {}. Raw response: {:?}",
                        file_path, position.line, e, result
                    );
                    return Err(e.into());
                }
            }
        };

        debug!(
            "Returning {} references for {}:{}",
            references.len(),
            file_path,
            position.line
        );
        Ok(references)
    }

    async fn call_hierarchy_incoming_calls(
        &mut self,
        item: CallHierarchyItem,
    ) -> Result<Vec<CallHierarchyIncomingCall>, Box<dyn Error + Send + Sync>> {
        debug!(
            "call_hierarchy_incoming_calls: Starting for item: name={}, uri={}, range={:?}",
            item.name, item.uri, item.selection_range
        );

        let params = CallHierarchyIncomingCallsParams {
            item,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let result = self
            .send_request(
                "callHierarchy/incomingCalls",
                Some(serde_json::to_value(params)?),
            )
            .await?;

        debug!(
            "call_hierarchy_incoming_calls: Raw response from server: {:#?}",
            result
        );

        if result.is_null() {
            debug!("call_hierarchy_incoming_calls: Server returned null response");
            Ok(vec![])
        } else {
            match serde_json::from_value::<Vec<CallHierarchyIncomingCall>>(result.clone()) {
                Ok(calls) => {
                    debug!(
                        "call_hierarchy_incoming_calls: Successfully parsed {} calls",
                        calls.len()
                    );
                    Ok(calls)
                }
                Err(e) => {
                    error!(
                        "call_hierarchy_incoming_calls: Failed to parse response: {}. Raw response: {:?}",
                        e, result
                    );
                    Err(e.into())
                }
            }
        }
    }

    async fn call_hierarchy_outgoing_calls(
        &mut self,
        item: CallHierarchyItem,
    ) -> Result<Vec<CallHierarchyOutgoingCall>, Box<dyn Error + Send + Sync>> {
        debug!(
            "call_hierarchy_outgoing_calls: Starting for item: name={}, uri={}, range={:?}",
            item.name, item.uri, item.selection_range
        );

        let params = CallHierarchyOutgoingCallsParams {
            item,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let result = self
            .send_request(
                "callHierarchy/outgoingCalls",
                Some(serde_json::to_value(params)?),
            )
            .await?;

        debug!(
            "call_hierarchy_outgoing_calls: Raw response from server: {:#?}",
            result
        );

        if result.is_null() {
            debug!("call_hierarchy_outgoing_calls: Server returned null response");
            Ok(vec![])
        } else {
            match serde_json::from_value::<Vec<CallHierarchyOutgoingCall>>(result.clone()) {
                Ok(calls) => {
                    debug!(
                        "call_hierarchy_outgoing_calls: Successfully parsed {} calls",
                        calls.len()
                    );
                    Ok(calls)
                }
                Err(e) => {
                    error!(
                        "call_hierarchy_outgoing_calls: Failed to parse response: {}. Raw response: {:?}",
                        e, result
                    );
                    Err(e.into())
                }
            }
        }
    }

    async fn prepare_call_hierarchy(
        &mut self,
        file_path: &str,
        position: Position,
        use_manual_hierarchy: bool,
    ) -> Result<Vec<CallHierarchyItem>, Box<dyn Error + Send + Sync>> {
        debug!(
            "prepare_call_hierarchy: Starting with file={}, position={:?}, manual={}",
            file_path, position, use_manual_hierarchy
        );
        
        if !use_manual_hierarchy {
            debug!("prepare_call_hierarchy: Using LSP server implementation");
            let needs_open = {
                let workspace_documents = self.get_workspace_documents();
                let config = workspace_documents.get_did_open_configuration();
                let is_open = workspace_documents.is_did_open_document(file_path);
                debug!(
                    "prepare_call_hierarchy: Document status - config={:?}, is_open={}",
                    config, is_open
                );
                config == DidOpenConfiguration::Lazy && !is_open
            };

            // Always read the document text for diagnostics
            let document_text = self
                .get_workspace_documents()
                .read_text_document(&PathBuf::from(file_path), None)
                .await?;

            // For Go, if we're on a method call (x.y), adjust position to the method name
            let lines: Vec<&str> = document_text.lines().collect();
            let adjusted_position = if let Some(line) = lines.get(position.line as usize) {
                let before_cursor = &line[..position.character as usize];
                if let Some(dot_pos) = before_cursor.rfind('.') {
                    // We're after a dot, use the position right after the dot
                    Position {
                        line: position.line,
                        character: (dot_pos + 1) as u32,
                    }
                } else {
                    position
                }
            } else {
                position
            };

            // Log the content around the position
            if let Some(line) = lines.get(position.line as usize) {
                debug!(
                    "prepare_call_hierarchy: Content at position - Line {}: {:?}",
                    position.line, line
                );
                if position.line > 0 {
                    if let Some(prev_line) = lines.get(position.line as usize - 1) {
                        debug!("prepare_call_hierarchy: Previous line: {:?}", prev_line);
                    }
                }
                if let Some(next_line) = lines.get(position.line as usize + 1) {
                    debug!("prepare_call_hierarchy: Next line: {:?}", next_line);
                }
                debug!(
                    "prepare_call_hierarchy: Adjusted position from {:?} to {:?}",
                    position, adjusted_position
                );
            }

            if needs_open {
                debug!("prepare_call_hierarchy: Opening document {}", file_path);

                self.text_document_did_open(TextDocumentItem {
                    uri: Url::from_file_path(file_path).unwrap(),
                    language_id: detect_language_string(file_path)?,
                    version: 1,
                    text: document_text.clone(),
                })
                .await?;

                self.get_workspace_documents()
                    .add_did_open_document(file_path);
            }

            // For Go, if we're on a method call (x.y), adjust position to the method name
            let adjusted_position = if let Some(line) = document_text.lines().nth(position.line as usize) {
                let before_cursor = &line[..position.character as usize];
                if let Some(dot_pos) = before_cursor.rfind('.') {
                    // We're after a dot, use the position right after the dot
                    Position {
                        line: position.line,
                        character: (dot_pos + 1) as u32,
                    }
                } else {
                    position
                }
            } else {
                position
            };

            debug!(
                "prepare_call_hierarchy: Adjusted position from {:?} to {:?}",
                position, adjusted_position
            );

            let params = CallHierarchyPrepareParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: Url::from_file_path(file_path).map_err(|_| "Invalid file path")?,
                    },
                    position: adjusted_position,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            };

            debug!(
                "prepare_call_hierarchy: Sending request with params: {:?}",
                params
            );

            // Request diagnostics for the file
            let diagnostic_params = serde_json::json!({
                "textDocument": {
                    "uri": Url::from_file_path(file_path).map_err(|_| "Invalid file path")?
                }
            });
            
            if let Ok(diagnostics) = self
                .send_request(
                    "textDocument/diagnostic",
                    Some(diagnostic_params),
                )
                .await
            {
                debug!("File diagnostics: {:?}", diagnostics);
            }

            let result = self
                .send_request(
                    "textDocument/prepareCallHierarchy",
                    Some(serde_json::to_value(params)?),
                )
                .await?;
                
            debug!(
                "prepare_call_hierarchy: Raw response from server: {:#?}",
                result
            );

            // Get hover information for the position
            let hover_params = serde_json::json!({
                "textDocument": {
                    "uri": Url::from_file_path(file_path).map_err(|_| "Invalid file path")?
                },
                "position": {
                    "line": position.line,
                    "character": position.character
                }
            });

            if let Ok(hover_info) = self
                .send_request(
                    "textDocument/hover",
                    Some(hover_params),
                )
                .await
            {
                debug!("Hover information at position: {:?}", hover_info);
            }

            if result.is_null() {
                debug!("prepare_call_hierarchy: Server returned null response");
                Ok(vec![])
            } else {
                match serde_json::from_value::<Vec<CallHierarchyItem>>(result.clone()) {
                    Ok(items) => {
                        debug!(
                            "prepare_call_hierarchy: Successfully parsed {} items from response",
                            items.len()
                        );
                        for (i, item) in items.iter().enumerate() {
                            debug!(
                                "prepare_call_hierarchy: Item {}: name={}, kind={:?}, range={:?}",
                                i, item.name, item.kind, item.selection_range
                            );
                        }
                        Ok(items)
                    }
                    Err(e) => {
                        debug!(
                            "prepare_call_hierarchy: Failed to parse response: {}. Raw response: {:?}",
                            e, result
                        );
                        Ok(vec![])
                    }
                }
            }
        } else {
            debug!("prepare_call_hierarchy: Using manual implementation");
            debug!(
                "Manually preparing call hierarchy for file: {}, position: {:?}",
                file_path, position
            );

            // Get package info for the file
            let pkg = self.get_narrowest_package(file_path).await?;
            debug!("Got package: {:#?}", pkg);

            // Find the object at the given position (could be reference or definition)
            let obj = self
                .get_referenced_object(&pkg, file_path, position)
                .await?;
            debug!("Found object at position: {:#?}", obj);

            if let Some(obj) = obj {
                // If this is a reference (e.g. function call), look up its definition
                let definition_obj = if obj.is_reference {
                    debug!("Found reference, looking up definition");
                    // Look up the definition
                    let def_response = self.text_document_definition(
                        file_path,
                        Position {
                            line: obj.range.start.line,
                            character: obj.range.start.character,
                        },
                    ).await?;

                    match def_response {
                        GotoDefinitionResponse::Array(locations) if !locations.is_empty() => {
                            let def_location = &locations[0]; // Take first definition
                            debug!("Found definition at {:?}", def_location);

                            // Get package for definition file
                            let def_pkg =
                                self.get_narrowest_package(def_location.uri.path()).await?;

                            // Get object at definition location
                            self.get_referenced_object(
                                &def_pkg,
                                def_location.uri.path(),
                                def_location.range.start,
                            )
                            .await?
                        }
                        _ => {
                            debug!("No definition found for reference");
                            None
                        }
                    }
                } else {
                    Some(obj)
                };

                // Now verify the definition is a function and use it for the hierarchy item
                if let Some(def_obj) = definition_obj {
                    if !self.is_function_type(&def_obj) {
                        debug!("Definition is not a function type, returning empty result");
                        return Ok(vec![]);
                    }
                    debug!("Definition confirmed as function type");

                    let range = self.get_object_range(&def_obj)?;
                    debug!("Function range: {:?}", range);

                    // Create the CallHierarchyItem using the definition object
                    let filename = std::path::Path::new(&def_obj.file_path)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy();
                    let detail = format!("{} • {}", def_obj.package_path, filename);

                    let item = CallHierarchyItem {
                        name: def_obj.name,
                        kind: lsp_types::SymbolKind::FUNCTION,
                        tags: None,
                        detail: Some(detail),
                        uri: Url::from_file_path(&def_obj.file_path)
                            .map_err(|_| "Invalid file path")?,
                        range,
                        selection_range: range,
                        data: None,
                    };
                    debug!("Created CallHierarchyItem: {:?}", item);
                    Ok(vec![item])
                } else {
                    debug!("No valid definition found, returning empty result");
                    Ok(vec![])
                }
            } else {
                debug!("No function found at position, returning empty result");
                Ok(vec![])
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

    // Helper functions for manual call hierarchy implementation
    async fn get_narrowest_package(
        &mut self,
        file_path: &str,
    ) -> Result<Package, Box<dyn Error + Send + Sync>> {
        let path = PathBuf::from(file_path);
        let mut current_dir = path.parent().ok_or("Invalid file path")?;

        // Start with the immediate directory
        let mut package_path = current_dir.to_str().unwrap_or("").to_string();

        // Get the language handler
        let lang_str = detect_language_string(file_path)?;
        let handler = crate::utils::call_hierarchy::get_call_hierarchy_handler(&lang_str)
            .ok_or_else(|| format!("No call hierarchy handler for language: {}", lang_str))?;

        // Walk up the directory tree looking for package identifiers
        while let Some(parent) = current_dir.parent() {
            if handler.is_package_root(parent) {
                package_path = parent.to_string_lossy().to_string();
                break;
            }
            current_dir = parent;
        }

        Ok(Package { path: package_path })
    }

    async fn get_referenced_object(
        &mut self,
        pkg: &Package,
        file_path: &str,
        pos: Position,
    ) -> Result<Option<Object>, Box<dyn Error + Send + Sync>> {
        debug!(
            "get_referenced_object: Starting for file {} at position {:?}",
            file_path, pos
        );
        debug!("get_referenced_object: Using package path: {}", pkg.path);

        // Read source file
        let source = self
            .get_workspace_documents()
            .read_text_document(&PathBuf::from(file_path), None)
            .await?;
        let source = Arc::new(source);
        debug!(
            "get_referenced_object: Read source file, length: {}",
            source.len()
        );

        // Initialize parser with language
        let mut parser = Parser::new();
        let lang_str = detect_language_string(file_path)?;
        debug!("get_referenced_object: Detected language: {}", lang_str);

        let handler = crate::utils::call_hierarchy::get_call_hierarchy_handler(&lang_str)
            .ok_or_else(|| {
                debug!("get_referenced_object: Unsupported language: {}", lang_str);
                "Unsupported language"
            })?;
        debug!("get_referenced_object: Configuring parser for {}", lang_str);
        handler.configure_parser(&mut parser)?;

        // Parse the file
        let tree = Arc::new(
            parser
                .parse(&*source, None)
                .ok_or("Failed to parse source")?,
        );
        debug!("get_referenced_object: Successfully parsed source tree");

        // Convert LSP position to tree-sitter Point
        let point = Point::new(pos.line as usize, pos.character as usize);
        debug!(
            "get_referenced_object: Converted LSP position to tree-sitter point: {:?}",
            point
        );

        // Debug: Print the source line we're looking at
        let line = source.lines().nth(point.row).unwrap_or("");
        debug!(
            "get_referenced_object: Looking at line {}: {:?}",
            point.row, line
        );
        debug!("get_referenced_object: Target column: {}", point.column);

        // Find the most specific named node at the position
        let initial_node = tree
            .root_node()
            .named_descendant_for_point_range(point, point)
            .ok_or("No node found at position")?;

        debug!(
            "get_referenced_object: Found node: kind={}, text={:?}",
            initial_node.kind(),
            source[initial_node.byte_range()].to_string()
        );

        // Get the appropriate query based on language
        let query_str = self.get_function_definition_query(file_path)?;
        let query = Query::new(&parser.language().unwrap(), query_str)?;
        let mut cursor = QueryCursor::new();
        debug!("get_referenced_object: Prepared query for finding definitions");

        // Create an Object for the node we found
        let obj = match initial_node.kind() {
            // If we're on an identifier that's being called (function reference)
            "identifier" | "property_identifier" | "field_identifier" | "self" => {
                let name = source[initial_node.byte_range()].to_string();
                debug!("get_referenced_object: Found function reference: {}", name);

                // Walk up the tree to find a call expression or function definition
                let mut current = initial_node;
                let mut found_call = false;
                let mut found_function = false;
                debug!(
                    "get_referenced_object: Starting node walk from: {}",
                    current.kind()
                );

                while let Some(parent) = current.parent() {
                    debug!(
                        "get_referenced_object: Walking up tree, current node: {}",
                        parent.kind()
                    );
                    if parent.kind() == "call_expression" || parent.kind() == "call" {
                        found_call = true;
                        debug!("get_referenced_object: Found call expression");
                        break;
                    } else if parent.kind() == "function_item" {
                        found_function = true;
                        debug!("get_referenced_object: Found function definition");
                        break;
                    }
                    current = parent;
                }

                if found_call || found_function {
                    debug!("get_referenced_object: Creating object for {}", if found_call { "function call" } else { "function definition" });
                    Some(Object {
                        name,
                        package_path: pkg.path.clone(),
                        file_path: file_path.to_string(),
                        range: match self.tree_sitter_to_lsp_range(&current, &source) {
                            Ok(r) => r,
                            Err(_e) => return Ok(None),
                        },
                        node_range: (current.start_byte(), current.end_byte()),
                        source: source.clone(),
                        tree: tree.clone(),
                        is_reference: found_call,
                    })
                } else if let Some(parent) = initial_node.parent() {
                    if parent.kind() == "function_definition" || parent.kind() == "method_definition" || parent.kind() == "function_item" {
                        debug!("get_referenced_object: Found function/method definition");
                        // Use the parent node (full function) for the range and node_range
                        Some(Object {
                            name,
                            package_path: pkg.path.clone(),
                            file_path: file_path.to_string(),
                            range: match self.tree_sitter_to_lsp_range(&parent, &source) {
                                Ok(r) => r,
                                Err(_e) => return Ok(None),
                            },
                            node_range: (parent.start_byte(), parent.end_byte()),
                            source: source.clone(),
                            tree: tree.clone(),
                            is_reference: false,
                        })
                    } else {
                        debug!(
                            "get_referenced_object: Node parent is: {}, not a call or definition",
                            parent.kind()
                        );
                        None
                    }
                } else {
                    debug!("get_referenced_object: No parent node found");
                    None
                }
            },
            // If we're directly on a definition node
            "function_definition"
            | "method_definition"
            | "function_declaration"
            | "class_definition"
            | "class_declaration" => {
                debug!(
                    "get_referenced_object: Directly on a definition node: {}",
                    initial_node.kind()
                );
                // Get the name from the definition
                let mut query_matches = cursor.matches(&query, initial_node, source.as_bytes());
                loop {
                    query_matches.advance();
                    match query_matches.get() {
                        Some(match_) => {
                            for capture in match_.captures {
                                if query.capture_names()[capture.index as usize].ends_with("_name") {
                                    let name = source[capture.node.byte_range()].to_string();
                                    debug!("get_referenced_object: Found definition name: {}", name);
                                    return Ok(Some(Object {
                                        name,
                                        package_path: pkg.path.clone(),
                                        file_path: file_path.to_string(),
                                        range: match self.tree_sitter_to_lsp_range(&initial_node, &source) {
                                            Ok(r) => r,
                                            Err(_e) => return Ok(None),
                                        },
                                        node_range: (initial_node.start_byte(), initial_node.end_byte()),
                                        source: source.clone(),
                                        tree: tree.clone(),
                                        is_reference: false,
                                    }));
                                }
                            }
                        },
                        None => break,
                    }
                }
                debug!("get_referenced_object: No name found in definition node");
                None
            }
            _ => {
                debug!(
                    "get_referenced_object: Unhandled node kind: {}",
                    initial_node.kind()
                );
                None
            }
        };

        debug!("get_referenced_object: Returning result: {:?}", obj);
        Ok(obj)
    }

    fn is_function_type(&self, obj: &Object) -> bool {
        // Get the node from the tree at the object's range
        let tree = &obj.tree;
        let node = tree
            .root_node()
            .descendant_for_byte_range(obj.node_range.0, obj.node_range.1)
            .unwrap_or(tree.root_node());

        // Get language handler and check node type
        debug!("checking node kind for node: {:?}", node);
        if let Ok(lang) = detect_language_string(&obj.file_path) {
            if let Some(handler) = crate::utils::call_hierarchy::get_call_hierarchy_handler(&lang) {
                return handler.is_function_type(node.kind());
            }
        }
        false
    }

    fn get_object_range(
        &self,
        obj: &Object,
    ) -> Result<lsp_types::Range, Box<dyn Error + Send + Sync>> {
        let source = &obj.source;

        // Pre-calculate line offsets for the entire source
        let mut line_offsets = Vec::new();
        let mut offset = 0;
        line_offsets.push(0); // First line starts at 0

        for line in source.split('\n') {
            offset += line.len() + 1; // +1 for \n
            line_offsets.push(offset);
        }

        // Binary search to find line number for a byte offset
        let find_line = |byte_offset: usize| -> (usize, usize) {
            match line_offsets.binary_search(&byte_offset) {
                Ok(line) => (line, 0), // Exactly at line start
                Err(line) => {
                    let line = if line > 0 { line - 1 } else { 0 };
                    let col = byte_offset - line_offsets[line];
                    (line, col)
                }
            }
        };

        // Convert byte offsets to UTF-16 code unit offsets for LSP
        let byte_to_utf16_col = |line_start: usize, byte_col: usize| {
            let line_str = if let Some((start, _end)) = source[line_start..].split_once('\n') {
                start
            } else {
                &source[line_start..]
            };

            if byte_col > line_str.len() {
                return byte_col; // Fallback for invalid offset
            }

            line_str[..byte_col]
                .chars()
                .map(|c| {
                    if c as u32 >= 0x10000 {
                        2 // Surrogate pair
                    } else {
                        1 // Single UTF-16 code unit
                    }
                })
                .sum()
        };

        // Calculate start position
        let (start_line, start_byte_col) = find_line(obj.node_range.0);
        let start_line_offset = line_offsets[start_line];
        let start_char = byte_to_utf16_col(start_line_offset, start_byte_col);

        // Calculate end position
        let (end_line, end_byte_col) = find_line(obj.node_range.1);
        let end_line_offset = line_offsets[end_line];
        let end_char = byte_to_utf16_col(end_line_offset, end_byte_col);

        Ok(lsp_types::Range {
            start: lsp_types::Position {
                line: start_line as u32,
                character: start_char as u32,
            },
            end: lsp_types::Position {
                line: end_line as u32,
                character: end_char as u32,
            },
        })
    }
    

    async fn find_function_calls(
        &mut self,
        obj: &Object,
    ) -> Result<Vec<lsp_types::Range>, Box<dyn Error + Send + Sync>> {
        // Query to find function calls in the AST
        let query_str = self.get_function_call_query(&obj.file_path)?;

        let query = Query::new(&obj.tree.language(), query_str)?;
        let mut cursor = QueryCursor::new();
        // Get the root node and find the node for our range
        let root_node = obj.tree.root_node();
        let node = root_node
            .descendant_for_byte_range(obj.node_range.0, obj.node_range.1)
            .ok_or("Failed to find node for range")?;
        let mut matches = cursor.matches(&query, node, obj.source.as_bytes());

        let mut ranges = Vec::new();
        loop {
            matches.advance();
            match matches.get() {
                Some(match_) => {
                    for capture in match_.captures {
                        if query.capture_names()[capture.index as usize] == "call" {
                            let node = capture.node;
                            let start_pos = self.tree_sitter_to_lsp_pos(&node, &obj.source)?;
                            let end_pos = self.tree_sitter_to_lsp_pos_end(&node, &obj.source)?;
                            ranges.push(lsp_types::Range::new(start_pos, end_pos));
                        }
                    }
                },
                None => break,
            }
        }

        Ok(ranges)
    }

    fn tree_sitter_to_lsp_pos(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> Result<Position, Box<dyn Error + Send + Sync>> {
        let start_byte = node.start_byte();
        let mut line = 0;
        let mut col = 0;

        for (i, c) in source.chars().enumerate() {
            if i == start_byte {
                break;
            }
            if c == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }

        Ok(Position::new(line as u32, col as u32))
    }

    fn tree_sitter_to_lsp_pos_end(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> Result<Position, Box<dyn Error + Send + Sync>> {
        let end_byte = node.end_byte();
        let mut line = 0;
        let mut col = 0;

        for (i, c) in source.chars().enumerate() {
            if i == end_byte {
                break;
            }
            if c == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }

        Ok(Position::new(line as u32, col as u32))
    }

    fn tree_sitter_to_lsp_range(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> Result<Range, Box<dyn Error + Send + Sync>> {
        let start = self.tree_sitter_to_lsp_pos(node, source)?;
        let end = self.tree_sitter_to_lsp_pos_end(node, source)?;
        Ok(Range::new(start, end))
    }

    fn get_function_call_query(
        &self,
        file_path: &str,
    ) -> Result<&'static str, Box<dyn Error + Send + Sync>> {
        let path = PathBuf::from(file_path);
        let lang = detect_language_string(path.to_str().ok_or("Invalid path")?)?;
        let handler = crate::utils::call_hierarchy::get_call_hierarchy_handler(&lang)
            .ok_or_else(|| format!("Unsupported language: {}", lang))?;
        Ok(handler.get_function_call_query())
    }

    fn get_function_definition_query(
        &self,
        file_path: &str,
    ) -> Result<&'static str, Box<dyn Error + Send + Sync>> {
        let lang = detect_language_string(file_path)?;
        let handler = crate::utils::call_hierarchy::get_call_hierarchy_handler(&lang)
            .ok_or_else(|| format!("Unsupported language: {}", lang))?;
        Ok(handler.get_function_definition_query())
    }
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
        // Base implementation does nothing
        // Specific language clients can override this if needed
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
}
