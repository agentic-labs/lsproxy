use std::path::Path;
use std::process::Stdio;

use async_trait::async_trait;
use lsp_types::InitializeParams;
use notify_debouncer_mini::DebouncedEvent;
use tokio::process::Command;
use tokio::sync::broadcast::Receiver;
use url::Url;

use crate::lsp::{JsonRpcHandler, LspClient, PendingRequests, ProcessHandler};

use crate::utils::workspace_documents::{
    DidOpenConfiguration, WorkspaceDocumentsHandler, DEFAULT_EXCLUDE_PATTERNS,
    TYPESCRIPT_AND_JAVASCRIPT_FILE_PATTERNS, TYPESCRIPT_AND_JAVASCRIPT_ROOT_FILES,
};

pub struct TypeScriptLanguageClient {
    process: ProcessHandler,
    json_rpc: JsonRpcHandler,
    workspace_documents: WorkspaceDocumentsHandler,
    pending_requests: PendingRequests,
}

#[async_trait]
impl LspClient for TypeScriptLanguageClient {
    fn get_process(&mut self) -> &mut ProcessHandler {
        &mut self.process
    }

    fn get_json_rpc(&mut self) -> &mut JsonRpcHandler {
        &mut self.json_rpc
    }

    fn get_root_files(&mut self) -> Vec<String> {
        TYPESCRIPT_AND_JAVASCRIPT_ROOT_FILES
            .iter()
            .map(|&s| s.to_owned())
            .collect()
    }

    fn get_pending_requests(&mut self) -> &mut PendingRequests {
        &mut self.pending_requests
    }

    fn get_workspace_documents(&mut self) -> &mut WorkspaceDocumentsHandler {
        &mut self.workspace_documents
    }

    async fn get_initialize_params(&mut self, root_path: String) -> InitializeParams {
        let capabilities = self.get_capabilities();
        InitializeParams {
            capabilities,
            root_uri: Some(Url::from_file_path(root_path).unwrap()),
            initialization_options: Some(serde_json::json!({
                "tsserver": {
                    "useSyntaxServer": "never"
                }
            })),
            ..Default::default()
        }
    }
}

impl TypeScriptLanguageClient {
    pub async fn new(
        root_path: &str,
        watch_events_rx: Receiver<DebouncedEvent>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let process = Command::new("typescript-language-server")
            .arg("--stdio")
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
            TYPESCRIPT_AND_JAVASCRIPT_FILE_PATTERNS
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            DEFAULT_EXCLUDE_PATTERNS
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            watch_events_rx,
            DidOpenConfiguration::Lazy,
        );
        Ok(Self {
            process: process_handler,
            json_rpc: json_rpc_handler,
            workspace_documents,
            pending_requests: PendingRequests::new(),
        })
    }
}
