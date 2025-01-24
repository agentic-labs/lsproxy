use std::{error::Error, path::Path, process::Stdio};

use async_trait::async_trait;
use log::debug;
use lsp_types::WorkspaceFolder;
use tokio::{process::Command, sync::broadcast::Receiver};

use crate::{lsp::{JsonRpcHandler, LspClient, PendingRequests, ProcessHandler}, utils::workspace_documents::{DidOpenConfiguration, WorkspaceDocumentsHandler, DEFAULT_EXCLUDE_PATTERNS, RUBY_FILE_PATTERNS, RUBY_ROOT_FILES}};

pub struct RubyClient {
    process: ProcessHandler,
    json_rpc: JsonRpcHandler,
    workspace_documents: WorkspaceDocumentsHandler,
    pending_requests: PendingRequests,
}

#[async_trait]
impl LspClient for RubyClient {
    fn get_process(&mut self) -> &mut ProcessHandler {
        &mut self.process
    }

    fn get_json_rpc(&mut self) -> &mut JsonRpcHandler {
        &mut self.json_rpc
    }

    fn get_root_files(&mut self) -> Vec<String> {
        RUBY_ROOT_FILES.iter().map(|&s| s.to_string()).collect()
    }

    fn get_workspace_documents(&mut self) -> &mut WorkspaceDocumentsHandler {
        &mut self.workspace_documents
    }

    fn get_pending_requests(&mut self) -> &mut PendingRequests {
        &mut self.pending_requests
    }


    async fn find_workspace_folders(
        &mut self,
        root_path: String,
    ) -> Result<Vec<WorkspaceFolder>, Box<dyn Error + Send + Sync>> {
        Ok(vec![])
    }
}

impl RubyClient {
    pub async fn new(
        root_path: &str,
        watch_events_rx: Receiver<notify_debouncer_mini::DebouncedEvent>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        debug!("Launching Ruby LSP in {:?}", root_path);

        let process = Command::new("ruby-lsp")
            .arg("--use-launcher")
            .current_dir(root_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let process_handler = ProcessHandler::new(process).await?;
        debug!("Ruby LSP process spawned successfully");

        let workspace_documents = WorkspaceDocumentsHandler::new(
            Path::new(root_path),
            RUBY_FILE_PATTERNS.iter().map(|&s| s.to_string()).collect(),
            DEFAULT_EXCLUDE_PATTERNS.iter().map(|&s| s.to_string()).collect(),
            watch_events_rx,
            DidOpenConfiguration::None,
        );

        let json_rpc_handler = JsonRpcHandler::new();

        Ok(Self {
            process: process_handler,
            json_rpc: json_rpc_handler,
            workspace_documents,
            pending_requests: PendingRequests::new(),
        })
    }
}
