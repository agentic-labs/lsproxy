use std::{error::Error, path::Path, process::Stdio};

use async_trait::async_trait;
use lsp_types::{
    ClientCapabilities, DocumentSymbolClientCapabilities, InitializeParams,
    TextDocumentClientCapabilities, WorkspaceFolder,
};
use notify_debouncer_mini::DebouncedEvent;
use tokio::process::Command;
use tokio::sync::broadcast::Receiver;

use crate::lsp::{JsonRpcHandler, LspClient, PendingRequests, ProcessHandler};

use crate::utils::workspace_documents::{
    WorkspaceDocumentsHandler, DEFAULT_EXCLUDE_PATTERNS, RUST_ANALYZER_FILE_PATTERNS,
    RUST_ANALYZER_ROOT_FILES,
};

pub struct RustAnalyzerClient {
    process: ProcessHandler,
    json_rpc: JsonRpcHandler,
    workspace_documents: WorkspaceDocumentsHandler,
    pending_requests: PendingRequests,
}

#[async_trait]
impl LspClient for RustAnalyzerClient {
    fn get_process(&mut self) -> &mut ProcessHandler {
        &mut self.process
    }

    fn get_json_rpc(&mut self) -> &mut JsonRpcHandler {
        &mut self.json_rpc
    }

    fn get_root_files(&mut self) -> Vec<String> {
        RUST_ANALYZER_ROOT_FILES
            .iter()
            .map(|&s| s.to_owned())
            .collect()
    }

    fn get_workspace_documents(&mut self) -> &mut WorkspaceDocumentsHandler {
        &mut self.workspace_documents
    }

    fn get_pending_requests(&mut self) -> &mut PendingRequests {
        &mut self.pending_requests
    }

    async fn setup_workspace(
        &mut self,
        _root_path: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // This is required for workspace features like go to definition to work
        self.send_request("rust-analyzer/reloadWorkspace", None)
            .await?;
        Ok(())
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
            // TODO THIS WILL CAUSE A BUNCH OF LINT ERRORS
            // We are doing this because we want to avoid looking up defs and refs in cargo registry
            // which is prohibitively slow
            initialization_options: Some(serde_json::json!({
                "cargo": {
                "sysroot": serde_json::Value::Null
                }
            })),
            ..Default::default()
        };
        params
    }
}

impl RustAnalyzerClient {
    pub async fn new(
        root_path: &str,
        watch_events_rx: Receiver<DebouncedEvent>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let process = Command::new("rust-analyzer")
            .current_dir(root_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let process_handler = ProcessHandler::new(process)
            .await
            .map_err(|e| format!("Failed to create ProcessHandler: {}", e))?;
        let json_rpc_handler = JsonRpcHandler::new();

        let workspace_documents = WorkspaceDocumentsHandler::new(
            Path::new(root_path),
            RUST_ANALYZER_FILE_PATTERNS
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            DEFAULT_EXCLUDE_PATTERNS
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            watch_events_rx,
        );

        Ok(Self {
            process: process_handler,
            json_rpc: json_rpc_handler,
            workspace_documents,
            pending_requests: PendingRequests::new(),
        })
    }
}
