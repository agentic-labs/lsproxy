use serde::{Deserialize, Serialize};
use tokio::fs;

use std::error::Error;
use std::path::Path;
use std::process::Stdio;

use crate::utils::file_utils::{absolute_path_to_relative_path_string, search_files};
use crate::utils::workspace_documents::WorkspaceDocuments;
use crate::{
    lsp::{JsonRpcHandler, LspClient, PendingRequests, ProcessHandler},
    utils::workspace_documents::{
        WorkspaceDocumentsHandler, CPP_ROOT_FILES, C_AND_CPP_FILE_PATTERNS,
        DEFAULT_EXCLUDE_PATTERNS,
    },
};
use async_trait::async_trait;
use fs::write;
use futures::future::try_join_all;
use log::{debug, info};
use lsp_types::TextDocumentItem;
use notify_debouncer_mini::DebouncedEvent;
use tokio::{process::Command, sync::broadcast::Receiver};
use url::Url;

pub struct ClangdClient {
    process: ProcessHandler,
    json_rpc: JsonRpcHandler,
    workspace_documents: WorkspaceDocumentsHandler,
    pending_requests: PendingRequests,
}

#[async_trait]
impl LspClient for ClangdClient {
    fn get_process(&mut self) -> &mut ProcessHandler {
        &mut self.process
    }

    fn get_json_rpc(&mut self) -> &mut JsonRpcHandler {
        &mut self.json_rpc
    }

    fn get_root_files(&mut self) -> Vec<String> {
        CPP_ROOT_FILES.iter().map(|s| s.to_string()).collect()
    }

    fn get_workspace_documents(&mut self) -> &mut WorkspaceDocumentsHandler {
        &mut self.workspace_documents
    }

    fn get_pending_requests(&mut self) -> &mut PendingRequests {
        &mut self.pending_requests
    }

    async fn setup_workspace(
        &mut self,
        root_path: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let compile_db_files = search_files(
            Path::new(root_path),
            vec!["**/compile_commands.json".to_string()],
            DEFAULT_EXCLUDE_PATTERNS
                .iter()
                .map(|s| s.to_string())
                .collect(),
        )?;

        if compile_db_files.is_empty() {
            // this is a workaround to avoid building the entire project
            let commands = generate_compile_commands(root_path.to_string())?;

            let json = serde_json::to_string_pretty(&commands)?;

            write(Path::new(root_path).join("compile_commands.json"), json).await?;

            debug!(
                "Generated compile_commands.json with {} entries",
                commands.len()
            );
        }

        let text_document_items = self
            .get_text_document_items_to_open_with_config(root_path)
            .await?;
        for item in text_document_items {
            self.text_document_did_open(item).await?;
        }
        Ok(())
    }
}

impl ClangdClient {
    pub async fn new(
        root_path: &str,
        watch_events_rx: Receiver<DebouncedEvent>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let process = Command::new("clangd")
            .arg("--log=error")
            .current_dir(root_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;

        let process_handler = ProcessHandler::new(process)
            .await
            .map_err(|e| format!("Failed to create ProcessHandler: {}", e))?;
        let json_rpc_handler = JsonRpcHandler::new();
        let workspace_documents = WorkspaceDocumentsHandler::new(
            Path::new(root_path),
            C_AND_CPP_FILE_PATTERNS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            DEFAULT_EXCLUDE_PATTERNS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            watch_events_rx,
        );
        let pending_requests = PendingRequests::new();

        Ok(Self {
            process: process_handler,
            json_rpc: json_rpc_handler,
            workspace_documents,
            pending_requests,
        })
    }

    pub async fn get_text_document_items_to_open_with_config(
        &mut self,
        _workspace_path: &str,
    ) -> Result<Vec<TextDocumentItem>, Box<dyn std::error::Error + Send + Sync>> {
        let file_paths = self.workspace_documents.list_files().await;

        let items = try_join_all(file_paths.into_iter().map(|file_path| {
            let workspace_documents = &self.workspace_documents;
            async move {
                let content = workspace_documents
                    .read_text_document(&file_path, None)
                    .await?;
                let uri =
                    Url::from_file_path(&file_path).map_err(|_| "Invalid file path".to_string())?;
                let language_id = match file_path.extension().and_then(|ext| ext.to_str()) {
                    Some("c") => "c",
                    _ => "cpp",
                }
                .to_string();
                info!("Processed file: {}", file_path.display());
                Ok::<TextDocumentItem, Box<dyn std::error::Error + Send + Sync>>(TextDocumentItem {
                    uri,
                    language_id: language_id.to_string(),
                    version: 1,
                    text: content,
                })
            }
        }))
        .await?;

        Ok(items)
    }
}

#[derive(Serialize, Deserialize)]
struct CompileCommand {
    directory: String,
    command: String,
    file: String,
}

fn is_cpp_file(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    matches!(ext, "hpp" | "hxx" | "cpp" | "cxx" | "cc")
}

fn generate_compile_commands(
    project_root: String,
) -> Result<Vec<CompileCommand>, Box<dyn Error + Send + Sync>> {
    let mut commands = Vec::new();

    // Search for both header and source files
    let all_files = search_files(
        Path::new(&project_root),
        C_AND_CPP_FILE_PATTERNS // This includes both .c/.cpp and .h/.hpp patterns
            .iter()
            .map(|s| s.to_string())
            .collect(),
        DEFAULT_EXCLUDE_PATTERNS
            .iter()
            .map(|s| s.to_string())
            .collect(),
    )?;

    for path in all_files {
        let relative_path = absolute_path_to_relative_path_string(&path)?;
        let compiler = if is_cpp_file(&path) {
            "/usr/bin/c++"
        } else {
            "/usr/bin/cc"
        };

        commands.push(CompileCommand {
            directory: project_root.clone(),
            // Add more comprehensive compiler flags
            command: format!(
                "{} -Wall -Wextra -I. -Iinclude -c {}",
                compiler, relative_path
            ),
            file: relative_path,
        });
    }

    debug!("Generated compile commands for {} files", commands.len());
    Ok(commands)
}
