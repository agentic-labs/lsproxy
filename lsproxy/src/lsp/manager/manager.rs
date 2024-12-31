use crate::api_types::{get_mount_dir, SupportedLanguages};
use crate::ast_grep::call_hierarchy::find_enclosing_function;
use crate::ast_grep::client::AstGrepClient;
use crate::ast_grep::types::AstGrepMatch;
use crate::lsp::client::LspClient;
use crate::lsp::languages::{
    ClangdClient, GoplsClient, JdtlsClient, JediClient, PhpactorClient, RustAnalyzerClient,
    TypeScriptLanguageClient,
};
use crate::utils::file_utils::{
    absolute_path_to_relative_path_string, detect_language, search_files,
};
use crate::utils::workspace_documents::{
    WorkspaceDocuments, C_AND_CPP_FILE_PATTERNS, DEFAULT_EXCLUDE_PATTERNS, GOLANG_FILE_PATTERNS,
    JAVA_FILE_PATTERNS, PHP_FILE_PATTERNS, PYTHON_FILE_PATTERNS, RUST_FILE_PATTERNS,
    TYPESCRIPT_AND_JAVASCRIPT_FILE_PATTERNS,
};
use log::{debug, error, warn};
use lsp_types::{CallHierarchyIncomingCall, CallHierarchyItem, CallHierarchyOutgoingCall, GotoDefinitionResponse, Location, Position, Range, SymbolKind};
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, DebouncedEvent};
use url::Url;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast::{channel, Sender};
use tokio::sync::Mutex;

pub struct Manager {
    lsp_clients: HashMap<SupportedLanguages, Arc<Mutex<Box<dyn LspClient>>>>,
    watch_events_sender: Sender<DebouncedEvent>,
    ast_grep: AstGrepClient,
}

impl Manager {
    pub async fn new(root_path: &str) -> Result<Self, Box<dyn Error>> {
        let (tx, _) = channel(100);
        let event_sender = tx.clone();
        let mut debouncer = new_debouncer(
            Duration::from_secs(2),
            move |res: DebounceEventResult| match res {
                Ok(events) => {
                    for event in events {
                        let _ = tx.send(event.clone());
                    }
                }
                Err(e) => error!("Debounce error: {:?}", e),
            },
        )
        .expect("Failed to create debouncer");

        // Watch the root path recursively
        debouncer
            .watcher()
            .watch(Path::new(root_path), RecursiveMode::Recursive)
            .expect("Failed to watch path");

        let ast_grep = AstGrepClient {
            config_path: String::from("/usr/src/ast_grep/sgconfig.yml"),
        };
        Ok(Self {
            lsp_clients: HashMap::new(),
            watch_events_sender: event_sender,
            ast_grep,
        })
    }

    /// Detects the languages in the workspace by searching for files that match the language server's file patterns, before LSPs are started.
    fn detect_languages_in_workspace(&self, root_path: &str) -> Vec<SupportedLanguages> {
        let mut lsps = Vec::new();
        for lsp in [
            SupportedLanguages::Python,
            SupportedLanguages::TypeScriptJavaScript,
            SupportedLanguages::Rust,
            SupportedLanguages::CPP,
            SupportedLanguages::Java,
            SupportedLanguages::Golang,
            SupportedLanguages::PHP,
        ] {
            let patterns = match lsp {
                SupportedLanguages::Python => PYTHON_FILE_PATTERNS
                    .iter()
                    .map(|&s| s.to_string())
                    .collect(),
                SupportedLanguages::TypeScriptJavaScript => TYPESCRIPT_AND_JAVASCRIPT_FILE_PATTERNS
                    .iter()
                    .map(|&s| s.to_string())
                    .collect(),
                SupportedLanguages::Rust => {
                    RUST_FILE_PATTERNS.iter().map(|&s| s.to_string()).collect()
                }
                SupportedLanguages::CPP => C_AND_CPP_FILE_PATTERNS
                    .iter()
                    .map(|&s| s.to_string())
                    .collect(),
                SupportedLanguages::Java => {
                    JAVA_FILE_PATTERNS.iter().map(|&s| s.to_string()).collect()
                }
                SupportedLanguages::Golang => GOLANG_FILE_PATTERNS
                    .iter()
                    .map(|&s| s.to_string())
                    .collect(),
                SupportedLanguages::PHP => {
                    PHP_FILE_PATTERNS.iter().map(|&s| s.to_string()).collect()
                }
            };
            if search_files(
                Path::new(root_path),
                patterns,
                DEFAULT_EXCLUDE_PATTERNS
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                true,
            )
            .map_err(|e| warn!("Error searching files: {}", e))
            .unwrap_or_default()
            .len()
                > 0
            {
                lsps.push(lsp);
            }
        }
        debug!("Starting LSPs: {:?}", lsps);
        lsps
    }

    pub async fn start_langservers(
        &mut self,
        workspace_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let lsps = self.detect_languages_in_workspace(workspace_path);
        for lsp in lsps {
            if self.get_client(lsp).is_some() {
                continue;
            }
            debug!("Starting {:?} LSP", lsp);
            let mut client: Box<dyn LspClient> = match lsp {
                SupportedLanguages::Python => Box::new(
                    JediClient::new(workspace_path, self.watch_events_sender.subscribe())
                        .await
                        .map_err(|e| e.to_string())?,
                ),
                SupportedLanguages::TypeScriptJavaScript => Box::new(
                    TypeScriptLanguageClient::new(
                        workspace_path,
                        self.watch_events_sender.subscribe(),
                    )
                    .await
                    .map_err(|e| e.to_string())?,
                ),
                SupportedLanguages::Rust => Box::new(
                    RustAnalyzerClient::new(workspace_path, self.watch_events_sender.subscribe())
                        .await
                        .map_err(|e| e.to_string())?,
                ),
                SupportedLanguages::CPP => Box::new(
                    ClangdClient::new(workspace_path, self.watch_events_sender.subscribe())
                        .await
                        .map_err(|e| e.to_string())?,
                ),
                SupportedLanguages::Java => Box::new(
                    JdtlsClient::new(workspace_path, self.watch_events_sender.subscribe())
                        .await
                        .map_err(|e| e.to_string())?,
                ),
                SupportedLanguages::Golang => Box::new(
                    GoplsClient::new(workspace_path, self.watch_events_sender.subscribe())
                        .await
                        .map_err(|e| e.to_string())?,
                ),
                SupportedLanguages::PHP => Box::new(
                    PhpactorClient::new(workspace_path, self.watch_events_sender.subscribe())
                        .await
                        .map_err(|e| e.to_string())?,
                ),
            };
            client
                .initialize(workspace_path.to_string())
                .await
                .map_err(|e| e.to_string())?;
            debug!("Setting up workspace");
            client
                .setup_workspace(workspace_path)
                .await
                .map_err(|e| e.to_string())?;
            self.lsp_clients.insert(lsp, Arc::new(Mutex::new(client)));
        }
        Ok(())
    }

    pub async fn definitions_in_file_ast_grep(
        &self,
        file_path: &str,
    ) -> Result<Vec<AstGrepMatch>, LspManagerError> {
        let workspace_files = self.list_files().await?;
        if !workspace_files.iter().any(|f| f == file_path) {
            return Err(LspManagerError::FileNotFound(file_path.to_string()));
        }
        let full_path = get_mount_dir().join(&file_path);
        let full_path_str = full_path.to_str().unwrap_or_default();
        let ast_grep_result = self
            .ast_grep
            .get_file_symbols(full_path_str)
            .await
            .map_err(|e| LspManagerError::InternalError(format!("Symbol retrieval failed: {}", e)));
        ast_grep_result
    }

    pub async fn find_definition(
        &self,
        file_path: &str,
        position: Position,
    ) -> Result<GotoDefinitionResponse, LspManagerError> {
        let workspace_files = self.list_files().await.map_err(|e| {
            LspManagerError::InternalError(format!("Workspace file retrieval failed: {}", e))
        })?;
        if !workspace_files.iter().any(|f| f == file_path) {
            return Err(LspManagerError::FileNotFound(file_path.to_string()).into());
        }
        let full_path = get_mount_dir().join(&file_path);
        let full_path_str = full_path.to_str().unwrap_or_default();
        let lsp_type = detect_language(full_path_str).map_err(|e| {
            LspManagerError::InternalError(format!("Language detection failed: {}", e))
        })?;
        let client = self
            .get_client(lsp_type)
            .ok_or(LspManagerError::LspClientNotFound(lsp_type))?;
        let mut locked_client = client.lock().await;
        locked_client
            .text_document_definition(full_path_str, position)
            .await
            .map_err(|e| {
                LspManagerError::InternalError(format!("Definition retrieval failed: {}", e))
            })
    }

    pub fn get_client(
        &self,
        lsp_type: SupportedLanguages,
    ) -> Option<Arc<Mutex<Box<dyn LspClient>>>> {
        self.lsp_clients.get(&lsp_type).cloned()
    }

    pub async fn find_references(
        &self,
        file_path: &str,
        position: Position,
    ) -> Result<Vec<Location>, LspManagerError> {
        let workspace_files = self.list_files().await.map_err(|e| {
            LspManagerError::InternalError(format!("Workspace file retrieval failed: {}", e))
        })?;

        if !workspace_files.iter().any(|f| f == file_path) {
            return Err(LspManagerError::FileNotFound(file_path.to_string()));
        }

        let full_path = get_mount_dir().join(&file_path);
        let full_path_str = full_path.to_str().unwrap_or_default();
        let lsp_type = detect_language(full_path_str).map_err(|e| {
            LspManagerError::InternalError(format!("Language detection failed: {}", e))
        })?;
        let client = self
            .get_client(lsp_type)
            .ok_or(LspManagerError::LspClientNotFound(lsp_type))?;
        let mut locked_client = client.lock().await;

        locked_client
            .text_document_reference(full_path_str, position)
            .await
            .map_err(|e| {
                LspManagerError::InternalError(format!("Reference retrieval failed: {}", e))
            })
    }

    pub async fn list_files(&self) -> Result<Vec<String>, LspManagerError> {
        let mut files = Vec::new();
        for client in self.lsp_clients.values() {
            let mut locked_client = client.lock().await;
            files.extend(
                locked_client
                    .get_workspace_documents()
                    .list_files()
                    .await
                    .iter()
                    .filter_map(|f| Some(absolute_path_to_relative_path_string(f)))
                    .collect::<Vec<String>>(),
            );
        }
        files.sort();
        Ok(files)
    }

    pub async fn read_source_code(
        &self,
        file_path: &str,
        range: Option<Range>,
    ) -> Result<String, LspManagerError> {
        let client = self.get_client(detect_language(file_path)?).ok_or(
            LspManagerError::LspClientNotFound(detect_language(file_path)?),
        )?;
        let full_path = get_mount_dir().join(&file_path);
        let mut locked_client = client.lock().await;
        locked_client
            .get_workspace_documents()
            .read_text_document(&full_path, range)
            .await
            .map_err(|e| {
                LspManagerError::InternalError(format!("Source code retrieval failed: {}", e))
            })
    }

    pub async fn prepare_call_hierarchy(
        &self,
        file_path: &str,
        position: Position,
        use_manual_hierarchy: bool,
    ) -> Result<Vec<CallHierarchyItem>, LspManagerError> {
        debug!(
            "[PrepareCallHierarchy] Starting analysis for file={}, position={}:{}, mode={}",
            file_path,
            position.line,
            position.character,
            if use_manual_hierarchy {
                "manual"
            } else {
                "lsp"
            }
        );

        // List workspace files
        debug!("[PrepareCallHierarchy] Retrieving workspace files");
        let workspace_files = self.list_files().await.map_err(|e| {
            error!(
                "[PrepareCallHierarchy] Failed to retrieve workspace files: {}",
                e
            );
            LspManagerError::InternalError(format!("Workspace file retrieval failed: {}", e))
        })?;

        // Verify file exists
        if !workspace_files.iter().any(|f| f == file_path) {
            error!(
                "[PrepareCallHierarchy] File not found in workspace: {}",
                file_path
            );
            return Err(LspManagerError::FileNotFound(file_path.to_string()));
        }
        debug!("[PrepareCallHierarchy] File found in workspace");

        // Get full path and detect language
        let full_path = get_mount_dir().join(&file_path);
        let full_path_str = full_path.to_str().unwrap_or_default();
        debug!(
            "[PrepareCallHierarchy] Analyzing file at full path: {}",
            full_path_str
        );

        let lsp_type = detect_language(full_path_str).map_err(|e| {
            error!("[PrepareCallHierarchy] Language detection failed: {}", e);
            LspManagerError::InternalError(format!("Language detection failed: {}", e))
        })?;
        debug!("[PrepareCallHierarchy] Detected language: {:?}", lsp_type);

        // Get LSP client
        debug!(
            "[PrepareCallHierarchy] Getting LSP client for {:?}",
            lsp_type
        );
        let client = self.get_client(lsp_type).ok_or_else(|| {
            error!(
                "[PrepareCallHierarchy] No LSP client found for {:?}",
                lsp_type
            );
            LspManagerError::LspClientNotFound(lsp_type)
        })?;

        debug!("[PrepareCallHierarchy] Acquiring client lock");
        let mut locked_client = client.lock().await;
        debug!("[PrepareCallHierarchy] Client lock acquired");

        // Prepare call hierarchy
        debug!(
            "[PrepareCallHierarchy] Requesting call hierarchy from LSP client (mode={})",
            if use_manual_hierarchy {
                "manual"
            } else {
                "lsp"
            }
        );
        let result = locked_client
            .prepare_call_hierarchy(full_path_str, position, use_manual_hierarchy)
            .await;

        match &result {
            Ok(items) => {
                debug!(
                    "[PrepareCallHierarchy] Successfully retrieved {} items",
                    items.len()
                );
                if !items.is_empty() {
                    for (i, item) in items.iter().enumerate() {
                        debug!(
                        "[PrepareCallHierarchy] Item {}/{}: name='{}', kind={:?}, range={}:{}-{}:{}",
                        i + 1,
                        items.len(),
                        item.name,
                        item.kind,
                        item.range.start.line,
                        item.range.start.character,
                        item.range.end.line,
                        item.range.end.character
                    );
                    }
                }
            }
            Err(e) => {
                error!(
                    "[PrepareCallHierarchy] Failed to prepare call hierarchy: {}",
                    e
                );
            }
        }

        result.map_err(|e| {
            LspManagerError::InternalError(format!("Call hierarchy preparation failed: {}", e))
        })
    }
    pub async fn incoming_calls(
        &self,
        item: &CallHierarchyItem,
    ) -> Result<Vec<CallHierarchyIncomingCall>, LspManagerError> {
        debug!(
            "[IncomingCalls] Starting analysis for item: name='{}', uri={}, range={}:{}-{}:{}",
            item.name,
            item.uri,
            item.range.start.line,
            item.range.start.character,
            item.range.end.line,
            item.range.end.character
        );

        // Get LSP client for the definition's language
        let lsp_type = detect_language(item.uri.path()).map_err(|e| {
            error!("[IncomingCalls] Language detection failed: {}", e);
            LspManagerError::InternalError(format!("Language detection failed: {}", e))
        })?;
        let client = self.get_client(lsp_type).ok_or_else(|| {
            error!("[IncomingCalls] No LSP client found for {:?}", lsp_type);
            LspManagerError::LspClientNotFound(lsp_type)
        })?;

        debug!("[IncomingCalls] Acquiring client lock");
        let mut locked_client = client.lock().await;
        debug!("[IncomingCalls] Client lock acquired");

        // Use LSP's callHierarchy/incomingCalls request
        let result = locked_client
            .call_hierarchy_incoming_calls(item.clone())
            .await
            .map_err(|e| {
                error!("[IncomingCalls] Failed to get incoming calls: {}", e);
                LspManagerError::InternalError(format!("Incoming calls retrieval failed: {}", e))
            })?;

        debug!("[IncomingCalls] Found {} incoming calls", result.len());
        Ok(result)
    }

    pub async fn outgoing_calls(
        &self,
        item: &CallHierarchyItem,
        use_manual_hierarchy: bool,
    ) -> Result<Vec<CallHierarchyOutgoingCall>, LspManagerError> {
        debug!(
            "[OutgoingCalls] Starting analysis for item: name='{}', uri={}, range={}:{}-{}:{}, mode={}",
            item.name,
            item.uri,
            item.range.start.line,
            item.range.start.character,
            item.range.end.line,
            item.range.end.character,
            if use_manual_hierarchy {
                "manual"
            } else {
                "lsp"
            }
        );

        // Get LSP client for the definition's language
        let lsp_type = detect_language(item.uri.path()).map_err(|e| {
            error!("[OutgoingCalls] Language detection failed: {}", e);
            LspManagerError::InternalError(format!("Language detection failed: {}", e))
        })?;
        let client = self.get_client(lsp_type).ok_or_else(|| {
            error!("[OutgoingCalls] No LSP client found for {:?}", lsp_type);
            LspManagerError::LspClientNotFound(lsp_type)
        })?;

        debug!("[OutgoingCalls] Acquiring client lock");
        let mut locked_client = client.lock().await;
        debug!("[OutgoingCalls] Client lock acquired");

        // Use LSP's callHierarchy/outgoingCalls request
        let result = locked_client
            .call_hierarchy_outgoing_calls(item.clone())
            .await
            .map_err(|e| {
                error!("[OutgoingCalls] Failed to get outgoing calls: {}", e);
                LspManagerError::InternalError(format!("Outgoing calls retrieval failed: {}", e))
            })?;

        debug!("[OutgoingCalls] Found {} outgoing calls", result.len());
        Ok(result)
    }
}

#[derive(Debug)]
pub enum LspManagerError {
    FileNotFound(String),
    LspClientNotFound(SupportedLanguages),
    InternalError(String),
    UnsupportedFileType(String),
}

impl fmt::Display for LspManagerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LspManagerError::FileNotFound(path) => {
                write!(f, "File '{}' not found in workspace", path)
            }
            LspManagerError::LspClientNotFound(lang) => {
                write!(f, "LSP client not found for {:?}", lang)
            }
            LspManagerError::InternalError(msg) => write!(f, "Internal error: {}", msg),
            LspManagerError::UnsupportedFileType(path) => {
                write!(f, "Unsupported file type: {}", path)
            }
        }
    }
}

impl std::error::Error for LspManagerError {}
