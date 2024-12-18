use crate::lsp::json_rpc::JsonRpc;
use crate::lsp::process::Process;
use crate::lsp::{ExpectedMessageKey, JsonRpcHandler, ProcessHandler};
use tree_sitter::{Parser, Query, QueryCursor, Tree, Point};
use std::sync::Arc;
use std::path::{Path, PathBuf};
use lsp_types::{Location, Url, Position, Range, CallHierarchyItem, CallHierarchyIncomingCall, CallHierarchyOutgoingCall};
use tree_sitter_python;
use tree_sitter_typescript;
use crate::utils::file_utils::{detect_language_string, search_directories};

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
    pub range: Range,
    pub node_range: (usize, usize), // start_byte, end_byte
    pub source: Arc<String>,
    pub tree: Arc<Tree>,
}
use async_trait::async_trait;
use log::{debug, error, warn};
use lsp_types::{
    CallHierarchyIncomingCallsParams, 
     CallHierarchyOutgoingCallsParams, CallHierarchyPrepareParams,
    ClientCapabilities, DidOpenTextDocumentParams, DocumentSymbolClientCapabilities,
    GotoDefinitionParams, GotoDefinitionResponse, InitializeParams, InitializeResult, 
    PartialResultParams,  PublishDiagnosticsClientCapabilities, ReferenceContext,
    ReferenceParams, TagSupport, TextDocumentClientCapabilities, TextDocumentIdentifier,
    TextDocumentItem, TextDocumentPositionParams,  WorkDoneProgressParams, WorkspaceFolder,
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
            "Requesting goto definition for {}, line {}, character {}",
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

        let references: Vec<Location> = serde_json::from_value(result)?;
        debug!("Received references response");
        Ok(references)
    }

    async fn prepare_call_hierarchy(
        &mut self,
        file_path: &str,
        position: Position,
        use_manual_hierarchy: bool,
    ) -> Result<Vec<CallHierarchyItem>, Box<dyn Error + Send + Sync>> {
        if !use_manual_hierarchy {
            // Try LSP server implementation first
            let needs_open = {
                let workspace_documents = self.get_workspace_documents();
                workspace_documents.get_did_open_configuration() == DidOpenConfiguration::Lazy
                    && !workspace_documents.is_did_open_document(file_path)
            };

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

            let params = CallHierarchyPrepareParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: Url::from_file_path(file_path).map_err(|_| "Invalid file path")?,
                    },
                    position,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            };

            let result = self
                .send_request(
                    "textDocument/prepareCallHierarchy",
                    Some(serde_json::to_value(params)?),
                )
                .await?;

            if result.is_null() {
                Ok(vec![])
            } else {
                let items: Vec<CallHierarchyItem> = serde_json::from_value(result)?;
                debug!("Received call hierarchy prepare response");
                Ok(items)
            }
        } else {
            debug!("Manually preparing call hierarchy for file: {}, position: {:?}", file_path, position);
            
            // Get package info for the file
            let pkg = self.get_narrowest_package(file_path).await?;
            debug!("Got package: {:#?}", pkg);
            
            // Find the function at the given position
            let obj = self.get_referenced_object(&pkg, file_path, position).await?;
            debug!("Found object at position: {:#?}", obj);
            
            if let Some(obj) = obj {
                // Verify it's a function
                if !self.is_function_type(&obj) {
                    debug!("Object is not a function type, returning empty result");
                    return Ok(vec![]);
                }
                debug!("Object confirmed as function type");

                let range = self.get_object_range(&obj)?;
                debug!("Function range: {:?}", range);
                
                // Create the CallHierarchyItem
                let filename = std::path::Path::new(file_path)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                let detail = format!("{} • {}", obj.package_path, filename);
                
                let item = CallHierarchyItem {
                    name: obj.name,
                    kind: lsp_types::SymbolKind::FUNCTION,
                    tags: None,
                    detail: Some(detail),
                    uri: Url::from_file_path(file_path).map_err(|_| "Invalid file path")?,
                    range,
                    selection_range: range,
                    data: None,
                };
                debug!("Created CallHierarchyItem: {:?}", item);
                
                Ok(vec![item])
            } else {
                debug!("No function found at position, returning empty result");
                Ok(vec![])
            }
        }
    }

    #[allow(unused)]
    async fn incoming_calls(
        &mut self,
        item: &CallHierarchyItem,
        use_manual_hierarchy: bool,
    ) -> Result<Vec<CallHierarchyIncomingCall>, Box<dyn Error + Send + Sync>> {
        if !use_manual_hierarchy {
            let params = CallHierarchyIncomingCallsParams {
                item: item.clone(),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            };

            let result = self
                .send_request(
                    "callHierarchy/incomingCalls",
                    Some(serde_json::to_value(params)?),
                )
                .await?;

            if result.is_null() {
                Ok(vec![])
            } else {
                let calls: Vec<CallHierarchyIncomingCall> = serde_json::from_value(result)?;
                debug!("Received incoming calls response");
                Ok(calls)
            }
        } else {
            // Manual implementation based on gopls
            let refs = self.get_references(&item.uri, item.range.start).await?;
            let mut incoming_calls = std::collections::HashMap::new();

            for ref_loc in refs {
                if let Ok(call_item) = self.get_enclosing_function(&ref_loc).await {
                    // Create a hashable key from Location components
                    let loc_key = (
                        call_item.uri.to_string(),
                        (call_item.range.start.line, call_item.range.start.character),
                        (call_item.range.end.line, call_item.range.end.character)
                    );

                    let entry = incoming_calls.entry(loc_key).or_insert_with(|| CallHierarchyIncomingCall {
                        from: call_item,
                        from_ranges: vec![],
                    });
                    entry.from_ranges.push(ref_loc.range);
                }
            }

            Ok(incoming_calls.into_values().collect())
        }
    }

    async fn outgoing_calls(
        &mut self,
        item: &CallHierarchyItem,
        use_manual_hierarchy: bool,
    ) -> Result<Vec<CallHierarchyOutgoingCall>, Box<dyn Error + Send + Sync>> {
        if !use_manual_hierarchy {
            let params = CallHierarchyOutgoingCallsParams {
                item: item.clone(),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            };

            let result = self
                .send_request(
                    "callHierarchy/outgoingCalls",
                    Some(serde_json::to_value(params)?),
                )
                .await?;

            if result.is_null() {
                Ok(vec![])
            } else {
                let calls: Vec<CallHierarchyOutgoingCall> = serde_json::from_value(result)?;
                debug!("Received outgoing calls response");
                Ok(calls)
            }
        } else {
            // Manual implementation based on gopls
            let decl_pkg = self.get_narrowest_package(item.uri.path()).await?;
            let decl_obj = self.get_referenced_object(&decl_pkg, item.uri.path(), item.range.start).await?;

            if let Some(decl_obj) = decl_obj {
                let mut outgoing_calls = std::collections::HashMap::new();
                let call_ranges = self.find_function_calls(&decl_obj).await?;

                for call_range in call_ranges {
                    let called_obj = self.get_referenced_object(
                        &decl_pkg,
                        item.uri.path(),
                        call_range.start,
                    ).await?;

                    if let Some(obj) = called_obj {
                        if self.is_function_type(&obj) {
                            let range = self.get_object_range(&obj)?;
                            let uri = Url::from_file_path(item.uri.path()).map_err(|_| "Invalid file path")?;

                            let call_item = CallHierarchyItem {
                                name: obj.name.clone(),
                                kind: lsp_types::SymbolKind::FUNCTION,
                                tags: None,
                                detail: Some(format!("{} • {}", obj.package_path, std::path::Path::new(item.uri.path()).file_name().unwrap().to_string_lossy())),
                                uri: uri.clone(),
                                range,
                                selection_range: range,
                                data: None,
                            };

                            // Create a hashable key from object name and location
                            let call_key = (
                                obj.name.clone(),
                                uri.to_string(),
                                (range.start.line, range.start.character),
                                (range.end.line, range.end.character)
                            );
                            let entry = outgoing_calls.entry(call_key).or_insert_with(|| CallHierarchyOutgoingCall {
                                to: call_item,
                                from_ranges: vec![],
                            });
                            entry.from_ranges.push(call_range);
                        }
                    }
                }

                Ok(outgoing_calls.into_values().collect())
            } else {
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
    async fn get_narrowest_package(&mut self, file_path: &str) -> Result<Package, Box<dyn Error + Send + Sync>> {
        // For Python and TypeScript, we consider the directory containing the file as the package
        // and walk up until we find a package identifier (package.json or __init__.py)
        let path = PathBuf::from(file_path);
        let mut current_dir = path.parent().ok_or("Invalid file path")?;
        
        // Start with the immediate directory
        let mut package_path = current_dir.to_string_lossy().to_string();
        
        // Walk up the directory tree looking for package identifiers
        while let Some(parent) = current_dir.parent() {
            let has_package_json = parent.join("package.json").exists();
            let has_init_py = parent.join("__init__.py").exists();
            
            if has_package_json || has_init_py {
                package_path = parent.to_string_lossy().to_string();
                break;
            }
            
            current_dir = parent;
        }
        
        Ok(Package {
            path: package_path,
        })
    }

    async fn get_referenced_object(&mut self, pkg: &Package, file_path: &str, pos: Position) 
        -> Result<Option<Object>, Box<dyn Error + Send + Sync>> {
        debug!("get_referenced_object: Starting for file {} at position {:?}", file_path, pos);
        debug!("get_referenced_object: Using package path: {}", pkg.path);

        // Read source file
        let source = self.get_workspace_documents()
            .read_text_document(&PathBuf::from(file_path), None)
            .await?;
        let source = Arc::new(source);
        debug!("get_referenced_object: Read source file, length: {}", source.len());

        // Initialize parser with language
        let mut parser = Parser::new();
        let lang_str = detect_language_string(file_path)?;
        debug!("get_referenced_object: Detected language: {}", lang_str);
        
        match lang_str.as_str() {
            "python" => {
                debug!("get_referenced_object: Configuring Python parser");
                parser.set_language(tree_sitter_python::language())?;
            }
            "typescript" | "javascript" => {
                debug!("get_referenced_object: Configuring TypeScript parser");
                parser.set_language(tree_sitter_typescript::language_typescript())?;
            }
            _ => {
                debug!("get_referenced_object: Unsupported language: {}", lang_str);
                return Err("Unsupported language".into())
            },
        };

        // Parse the file
        let tree = Arc::new(parser.parse(&*source, None)
            .ok_or("Failed to parse source")?);
        debug!("get_referenced_object: Successfully parsed source tree");

        // Convert LSP position to tree-sitter Point
        let point = Point::new(pos.line as usize, pos.character as usize);
        debug!("get_referenced_object: Converted LSP position to tree-sitter point: {:?}", point);

        // Debug: Print the source line we're looking at
        let line = source.lines().nth(point.row).unwrap_or("");
        debug!("get_referenced_object: Looking at line {}: {:?}", point.row, line);
        debug!("get_referenced_object: Target column: {}", point.column);

        // Find the most specific named node at the position
        let initial_node = tree.root_node()
            .named_descendant_for_point_range(point, point)
            .ok_or("No node found at position")?;
        
        debug!("get_referenced_object: Found node: kind={}, text={:?}", 
               initial_node.kind(),
               source[initial_node.byte_range()].to_string());

        // Get the appropriate query based on language
        let query_str = self.get_function_definition_query(file_path)?;
        let query = Query::new(parser.language().unwrap(), query_str)?;
        let mut cursor = QueryCursor::new();
        debug!("get_referenced_object: Prepared query for finding definitions");

        // Create an Object for the node we found
        let obj = match initial_node.kind() {
            // If we're on an identifier that's being called (function reference)
            "identifier" | "property_identifier" => {
                let name = source[initial_node.byte_range()].to_string();
                debug!("get_referenced_object: Found function reference: {}", name);
                
                // Verify this is a function call
                if let Some(parent) = initial_node.parent() {
                    if parent.kind() == "call" {
                        debug!("get_referenced_object: Confirmed as function call");
                        Some(Object {
                            name,
                            package_path: pkg.path.clone(),
                            range: self.tree_sitter_to_lsp_range(&initial_node, &source)?,
                            node_range: (initial_node.start_byte(), initial_node.end_byte()),
                            source: source.clone(),
                            tree: tree.clone(),
                        })
                    } else {
                        debug!("get_referenced_object: Not a function call, parent is: {}", parent.kind());
                        None
                    }
                } else {
                    debug!("get_referenced_object: No parent node found");
                    None
                }
            },
            // If we're directly on a definition node
            "function_definition" | "method_definition" |
            "function_declaration" | "class_definition" |
            "class_declaration" => {
                debug!("get_referenced_object: Directly on a definition node: {}", initial_node.kind());
                // Get the name from the definition
                for capture in cursor.matches(&query, initial_node, source.as_bytes()).flat_map(|m| m.captures) {
                    if query.capture_names()[capture.index as usize].ends_with("_name") {
                        let name = source[capture.node.byte_range()].to_string();
                        debug!("get_referenced_object: Found definition name: {}", name);
                        return Ok(Some(Object {
                            name,
                            package_path: pkg.path.clone(),
                            range: self.tree_sitter_to_lsp_range(&initial_node, &source)?,
                            node_range: (initial_node.start_byte(), initial_node.end_byte()),
                            source: source.clone(),
                            tree: tree.clone(),
                        }));
                    }
                }
                debug!("get_referenced_object: No name found in definition node");
                None
            },
            _ => {
                debug!("get_referenced_object: Unhandled node kind: {}", initial_node.kind());
                None
            }
        };

        debug!("get_referenced_object: Returning result: {:?}", obj.is_some());
        Ok(obj)
    }

    fn is_function_type(&self, obj: &Object) -> bool {
        // Get the node from the tree at the object's range
        let tree = &obj.tree;
        let node = tree.root_node()
            .descendant_for_byte_range(obj.node_range.0, obj.node_range.1)
            .unwrap_or(tree.root_node());

        // Check node type for Python and TypeScript function definitions
        debug!("checking node kind for node: {:?}",node);
        matches!(node.kind(), 
            "function_definition" |     // Python function
            "method_definition" |       // Python method
            "function_declaration" |    // TypeScript function
            "method_declaration" |      // TypeScript method
            "arrow_function" |          // TypeScript arrow function
            "function"                  // TypeScript function expression
        )
    }

    fn get_object_range(&self, obj: &Object) -> Result<lsp_types::Range, Box<dyn Error + Send + Sync>> {
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
            let line_str = if let Some((start, end)) = source[line_start..].split_once('\n') {
                start
            } else {
                &source[line_start..]
            };
            
            if byte_col > line_str.len() {
                return byte_col; // Fallback for invalid offset
            }
            
            line_str[..byte_col].chars()
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

    async fn get_references(&mut self, uri: &Url, pos: Position) -> Result<Vec<Location>, Box<dyn Error + Send + Sync>> {
        // First get the object at the position to find its name
        let pkg = self.get_narrowest_package(uri.path()).await?;
        let obj = self.get_referenced_object(&pkg, uri.path(), pos).await?;
        
        if let Some(obj) = obj {
            let mut references = Vec::new();
            let workspace_docs = self.get_workspace_documents();
            
            // Search for references in all workspace files
            for file_path in workspace_docs.list_files().await {
                let source = workspace_docs.read_text_document(&PathBuf::from(&file_path), None).await?;
                
                // Parse the file with tree-sitter
                let file_path_str = file_path.to_str().ok_or("Invalid file path")?;
                let mut parser = match detect_language_string(file_path_str)?.as_str() {
                    "python" => {
                        let mut parser = Parser::new();
                        parser.set_language(tree_sitter_python::language())?;
                        parser
                    },
                    "typescript" | "javascript" => {
                        let mut parser = Parser::new();
                        parser.set_language(tree_sitter_typescript::language_typescript())?;
                        parser
                    },
                    _ => continue,
                };
                
                let tree = parser.parse(&source, None).ok_or("Failed to parse file")?;
                
                // Create a query to find references to the function
                let query_text = match detect_language_string(file_path_str)?.as_str() {
                    "python" => format!(
                        "(call function: (identifier) @ref (#eq? @ref \"{}\"))", 
                        obj.name
                    ),
                    "typescript" | "javascript" => format!(
                        "(call_expression function: (identifier) @ref (#eq? @ref \"{}\"))",
                        obj.name
                    ),
                    _ => continue,
                };
                
                let query = Query::new(
                    parser.language().ok_or("Parser language not set")?,
                    &query_text
                )?;
                let mut cursor = QueryCursor::new();
                
                // Find all matches in the file
                for m in cursor.matches(&query, tree.root_node(), source.as_bytes()) {
                    for capture in m.captures {
                        let node = capture.node;
                        let start_pos = node.start_position();
                        let end_pos = node.end_position();
                        
                        references.push(Location {
                            uri: Url::from_file_path(&file_path).map_err(|_| "Invalid file path")?,
                            range: Range {
                                start: Position {
                                    line: start_pos.row as u32,
                                    character: start_pos.column as u32,
                                },
                                end: Position {
                                    line: end_pos.row as u32,
                                    character: end_pos.column as u32,
                                },
                            },
                        });
                    }
                }
            }
            
            Ok(references)
        } else {
            Ok(Vec::new())
        }
    }

    async fn get_enclosing_function(&mut self, loc: &Location) -> Result<CallHierarchyItem, Box<dyn Error + Send + Sync>> {
        let source = self.get_workspace_documents()
            .read_text_document(&PathBuf::from(loc.uri.path()), None)
            .await?;

        let mut parser = Parser::new();
        let language = match detect_language_string(loc.uri.path())?.as_str() {
            "python" => tree_sitter_python::language(),
            "typescript" | "javascript" => tree_sitter_typescript::language_typescript(),
            _ => return Err("Unsupported language".into()),
        };
        parser.set_language(language)?;

        let tree = parser.parse(&source, None)
            .ok_or("Failed to parse source")?;

        // Query to find the enclosing function
        let query_str = match detect_language_string(loc.uri.path())?.as_str() {
            "python" => r#"
                (function_definition
                  name: (identifier) @func_name
                ) @func_decl

                (class_definition
                  name: (identifier) @class_name
                  body: (block 
                    (function_definition
                      name: (identifier) @func_name) @func_decl)
                )
            "#,
            "typescript" | "javascript" => r#"
                (function_declaration
                  name: (identifier) @func_name
                ) @func_decl

                (method_definition
                  name: (property_identifier) @func_name
                ) @func_decl

                (class_declaration
                  name: (type_identifier) @class_name
                  body: (class_body
                    (method_definition
                      name: (property_identifier) @func_name) @func_decl)
                )

                (arrow_function
                  name: (identifier) @func_name
                ) @func_decl
            "#,
            _ => return Err("Unsupported language".into()),
        };

        let query = Query::new(language, query_str)?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

        // Find the function that contains our location
        for match_ in matches {
            for capture in match_.captures {
                if query.capture_names()[capture.index as usize] == "func_decl" {
                    let func_node = capture.node;
                    let func_start = self.tree_sitter_to_lsp_pos(&func_node, &source)?;
                    let func_end = self.tree_sitter_to_lsp_pos_end(&func_node, &source)?;
                    let func_range = lsp_types::Range::new(func_start, func_end);

                    // Check if this function contains our location
                    if self.range_contains(&func_range, &loc.range) {
                        // Find the function name
                        let name = match_.captures.iter()
                            .find(|c| query.capture_names()[c.index as usize] == "func_name")
                            .map(|c| {
                                let node = c.node;
                                source[node.byte_range()].to_string()
                            })
                            .unwrap_or_else(|| "anonymous".to_string());

                        return Ok(CallHierarchyItem {
                            name,
                            kind: lsp_types::SymbolKind::FUNCTION,
                            tags: None,
                            detail: Some(format!("{} • {}", 
                                std::path::Path::new(loc.uri.path()).parent().unwrap().to_string_lossy(),
                                std::path::Path::new(loc.uri.path()).file_name().unwrap().to_string_lossy()
                            )),
                            uri: loc.uri.clone(),
                            range: func_range,
                            selection_range: func_range,
                            data: None,
                        });
                    }
                }
            }
        }

        // If no enclosing function found, use the file scope
        Ok(CallHierarchyItem {
            name: "file_scope".to_string(),
            kind: lsp_types::SymbolKind::FILE,
            tags: None,
            detail: Some(format!("{}",
                std::path::Path::new(loc.uri.path()).file_name().unwrap().to_string_lossy()
            )),
            uri: loc.uri.clone(),
            range: lsp_types::Range::new(
                Position::new(0, 0),
                Position::new(u32::MAX, u32::MAX)
            ),
            selection_range: loc.range,
            data: None,
        })
    }

    fn range_contains(&self, outer: &lsp_types::Range, inner: &lsp_types::Range) -> bool {
        if outer.start.line > inner.start.line {
            return false;
        }
        if outer.end.line < inner.end.line {
            return false;
        }
        if outer.start.line == inner.start.line && outer.start.character > inner.start.character {
            return false;
        }
        if outer.end.line == inner.end.line && outer.end.character < inner.end.character {
            return false;
        }
        true
    }

    async fn find_function_calls(&mut self, obj: &Object) -> Result<Vec<lsp_types::Range>, Box<dyn Error + Send + Sync>> {
        // Query to find function calls in the AST
        let query_str = self.get_function_call_query(&obj.package_path)?;

        let query = Query::new(obj.tree.language(), query_str)?;
        let mut cursor = QueryCursor::new();
        // Get the root node and find the node for our range
        let root_node = obj.tree.root_node();
        let node = root_node.descendant_for_byte_range(obj.node_range.0, obj.node_range.1)
            .ok_or("Failed to find node for range")?;
        let matches = cursor.matches(&query, node, obj.source.as_bytes());

        let mut ranges = Vec::new();
        for match_ in matches {
            for capture in match_.captures {
                if query.capture_names()[capture.index as usize] == "call" {
                    let node = capture.node;
                    let start_pos = self.tree_sitter_to_lsp_pos(&node, &obj.source)?;
                    let end_pos = self.tree_sitter_to_lsp_pos_end(&node, &obj.source)?;
                    ranges.push(lsp_types::Range::new(start_pos, end_pos));
                }
            }
        }

        Ok(ranges)
    }

    fn tree_sitter_to_lsp_pos(&self, node: &tree_sitter::Node, source: &str) -> Result<Position, Box<dyn Error + Send + Sync>> {
        let start_byte = node.start_byte();
        let mut line = 0;
        let mut col = 0;
        let mut byte = 0;

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
            byte += c.len_utf8();
        }

        Ok(Position::new(line as u32, col as u32))
    }

    fn tree_sitter_to_lsp_pos_end(&self, node: &tree_sitter::Node, source: &str) -> Result<Position, Box<dyn Error + Send + Sync>> {
        let end_byte = node.end_byte();
        let mut line = 0;
        let mut col = 0;
        let mut byte = 0;

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
            byte += c.len_utf8();
        }

        Ok(Position::new(line as u32, col as u32))
    }

    fn tree_sitter_to_lsp_range(&self, node: &tree_sitter::Node, source: &str) -> Result<Range, Box<dyn Error + Send + Sync>> {
        let start = self.tree_sitter_to_lsp_pos(node, source)?;
        let end = self.tree_sitter_to_lsp_pos_end(node, source)?;
        Ok(Range::new(start, end))
    }

    fn get_function_call_query(&self, file_path: &str) -> Result<&'static str, Box<dyn Error + Send + Sync>> {
        match detect_language_string(file_path)?.as_str() {
            "python" => Ok(self.get_python_function_call_query()),
            "typescript" | "javascript" => Ok(self.get_typescript_function_call_query()),
            _ => Err("Unsupported language".into()),
        }
    }

    fn get_python_function_call_query(&self) -> &'static str {
        r#"
            ; Regular function calls
            (call
              function: (identifier) @func_name) @call

            ; Method calls
            (call
              function: (attribute
                object: (_)
                attribute: (identifier) @func_name)) @call

            ; Decorator calls
            (decorator
              decorator: (identifier) @func_name) @call

            ; Decorator with calls
            (decorator
              decorator: (call
                function: (identifier) @func_name)) @call

            ; Class constructor calls
            (call
              function: (identifier) @class_name
              [(argument_list) (keyword_argument)]) @call
        "#
    }

    fn get_typescript_function_call_query(&self) -> &'static str {
        r#"
            ; Regular function calls
            (call_expression
              function: (identifier) @func_name) @call

            ; Method calls
            (call_expression
              function: (member_expression
                property: (property_identifier) @func_name)) @call

            ; Constructor calls
            (new_expression
              constructor: (identifier) @class_name) @call

            ; Static method calls
            (call_expression
              function: (member_expression
                object: (identifier) @class_name
                property: (property_identifier) @func_name)) @call

            ; Immediately invoked function expressions (IIFE)
            ((call_expression
              function: (parenthesized_expression
                (arrow_function))) @call)

            ; Function calls with this
            (call_expression
              function: (member_expression
                object: (this)
                property: (property_identifier) @func_name)) @call
        "#
    }

    fn get_function_definition_query(&self, file_path: &str) -> Result<&'static str, Box<dyn Error + Send + Sync>> {
        match detect_language_string(file_path)?.as_str() {
            "python" => Ok(self.get_python_function_definition_query()),
            "typescript" | "javascript" => Ok(self.get_typescript_function_definition_query()),
            _ => Err("Unsupported language".into()),
        }
    }

    fn get_python_function_definition_query(&self) -> &'static str {
        r#"
            ; Regular functions
            (function_definition
              name: (identifier) @func_name
            ) @func_decl

            ; Class methods
            (class_definition
              name: (identifier) @class_name
              body: (block 
                (function_definition
                  name: (identifier) @func_name) @func_decl)
            )

            ; Async functions
            (function_definition
              "async"
              name: (identifier) @func_name
            ) @func_decl

            ; Lambda functions
            (lambda
              parameters: (lambda_parameters)?) @func_decl

            ; Decorated functions
            (decorated_definition
              definition: (function_definition
                name: (identifier) @func_name)) @func_decl
        "#
    }

    fn get_typescript_function_definition_query(&self) -> &'static str {
        r#"
            ; Regular functions
            (function_declaration
              name: (identifier) @func_name
            ) @func_decl

            ; Class methods
            (method_definition
              name: (property_identifier) @func_name
            ) @func_decl

            ; Class declarations with methods
            (class_declaration
              name: (type_identifier) @class_name
              body: (class_body
                (method_definition
                  name: (property_identifier) @func_name) @func_decl)
            )

            ; Arrow functions with names (variable declarations)
            (variable_declaration
              (variable_declarator
                name: (identifier) @func_name
                value: (arrow_function))) @func_decl

            ; Async functions
            (function_declaration
              "async"
              name: (identifier) @func_name
            ) @func_decl

            ; Generator functions
            (function_declaration
              "*"
              name: (identifier) @func_name
            ) @func_decl

            ; Async methods
            (method_definition
              "async"
              name: (property_identifier) @func_name
            ) @func_decl

            ; Constructor methods
            (method_definition
              "constructor"
              parameters: (formal_parameters)) @func_decl

            ; Static methods
            (method_definition
              "static"
              name: (property_identifier) @func_name
            ) @func_decl
        "#
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
