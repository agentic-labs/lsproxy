use std::process::Stdio;

use async_trait::async_trait;
use log::warn;
use lsp_types::WorkspaceSymbolResponse;
use tokio::process::Command;

use crate::lsp::{JsonRpcHandler, LspClient, ProcessHandler};

pub struct PythonClient {
    process: ProcessHandler,
    json_rpc: JsonRpcHandler,
}

#[async_trait]
impl LspClient for PythonClient {
    fn get_process(&mut self) -> &mut ProcessHandler {
        &mut self.process
    }

    fn get_json_rpc(&mut self) -> &mut JsonRpcHandler {
        &mut self.json_rpc
    }

    async fn workspace_symbols(
        &mut self,
        query: &str,
    ) -> Result<WorkspaceSymbolResponse, Box<dyn std::error::Error + Send + Sync>> {
        if query == "" || query == "*" {
            warn!(
                "Pyright doesn't support wildcards in workspace symbols query, expect empty result"
            );
        }
        LspClient::workspace_symbols(self, query).await
    }
}

impl PythonClient {
    pub async fn new(root_path: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let process = Command::new("pyright-langserver")
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

        Ok(Self {
            process: process_handler,
            json_rpc: json_rpc_handler,
        })
    }
}
