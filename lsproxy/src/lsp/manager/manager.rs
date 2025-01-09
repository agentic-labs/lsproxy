use crate::api_types::{get_mount_dir, FilePosition, Identifier, SupportedLanguages, Symbol};
use crate::ast_grep::client::AstGrepClient;
use crate::ast_grep::types::AstGrepMatch;
use crate::lsp::client::LspClient;
use crate::lsp::languages::{
    ClangdClient, GoplsClient, JdtlsClient, JediClient, PhpactorClient, RustAnalyzerClient,
    TypeScriptLanguageClient,
};
use crate::utils::file_utils::uri_to_relative_path_string;
use crate::utils::file_utils::{
    absolute_path_to_relative_path_string, detect_language, search_files,
};
use crate::utils::workspace_documents::{
    WorkspaceDocuments, C_AND_CPP_FILE_PATTERNS, DEFAULT_EXCLUDE_PATTERNS, GOLANG_FILE_PATTERNS,
    JAVA_FILE_PATTERNS, PHP_FILE_PATTERNS, PYTHON_FILE_PATTERNS, RUST_FILE_PATTERNS,
    TYPESCRIPT_AND_JAVASCRIPT_FILE_PATTERNS,
};
use log::{debug, error, warn};
use lsp_types::{GotoDefinitionResponse, Location, Position, Range};
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, DebouncedEvent};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
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
            symbol_config_path: String::from("/usr/src/ast_grep/symbol-config.yml"),
            identifier_config_path: String::from("/usr/src/ast_grep/identifier-config.yml"),
            reference_config_path: String::from("/usr/src/ast_grep/reference-config.yml"),
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
        if !workspace_files.contains(&file_path.to_string()) {
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

    pub async fn get_symbol_from_position(
        &self,
        file_path: &str,
        identifier_position: &lsp_types::Position,
    ) -> Result<Symbol, LspManagerError> {
        match self
            .ast_grep
            .get_symbol_from_position(file_path, identifier_position)
            .await
        {
            Ok(ast_grep_symbol) => Ok(Symbol::from(ast_grep_symbol)),
            Err(e) => Err(LspManagerError::InternalError(e.to_string())),
        }
    }

    pub async fn find_definition(
        &self,
        file_path: &str,
        position: Position,
    ) -> Result<GotoDefinitionResponse, LspManagerError> {
        let workspace_files = self.list_files().await.map_err(|e| {
            LspManagerError::InternalError(format!("Workspace file retrieval failed: {}", e))
        })?;
        if !workspace_files.contains(&file_path.to_string()) {
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
        let mut definition = locked_client
            .text_document_definition(full_path_str, position)
            .await
            .map_err(|e| {
                LspManagerError::InternalError(format!("Definition retrieval failed: {}", e))
            })?;

        // Sort the locations if there are multiple
        match &mut definition {
            GotoDefinitionResponse::Array(locations) => {
                locations.sort_by(|a, b| {
                    let path_a = uri_to_relative_path_string(&a.uri);
                    let path_b = uri_to_relative_path_string(&b.uri);
                    path_a
                        .cmp(&path_b)
                        .then(a.range.start.line.cmp(&b.range.start.line))
                        .then(a.range.start.character.cmp(&b.range.start.character))
                });
            }
            GotoDefinitionResponse::Link(links) => {
                links.sort_by(|a, b| {
                    let path_a = uri_to_relative_path_string(&a.target_uri);
                    let path_b = uri_to_relative_path_string(&b.target_uri);
                    path_a
                        .cmp(&path_b)
                        .then(a.target_range.start.line.cmp(&b.target_range.start.line))
                        .then(
                            a.target_range
                                .start
                                .character
                                .cmp(&b.target_range.start.character),
                        )
                });
            }
            _ => {}
        }
        Ok(definition)
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

        if !workspace_files.contains(&file_path.to_string()) {
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

    fn resolve_definition_chain<'a>(
        &'a self,
        file_path: &'a str,
        original_symbol: &'a Symbol,
        ast_match: &'a AstGrepMatch,
        client: &'a mut Box<dyn LspClient>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<GotoDefinitionResponse>, LspManagerError>> + 'a>>
    {
        Box::pin(async move {
            let full_path = get_mount_dir().join(file_path);
            let full_path_str = full_path.to_str().unwrap_or_default();

            // Get initial definition
            let definition = client
                .text_document_definition(full_path_str, lsp_types::Position::from(ast_match))
                .await
                .map_err(|e| {
                    LspManagerError::InternalError(format!("Definition retrieval failed: {}", e))
                })?;

            // Check if any definition locations are outside the original symbol's scope
            // OR if the symbol at this location is a function
            let has_external_definitions = match &definition {
                GotoDefinitionResponse::Scalar(loc) => {
                    let is_external = !original_symbol
                        .range
                        .contains(FilePosition::from(loc.clone()));
                    if !is_external {
                        // Check if internal symbol is a function
                        if let Ok(internal_symbol) = self
                            .get_symbol_from_position(
                                &uri_to_relative_path_string(&loc.uri),
                                &loc.range.start.into(),
                            )
                            .await
                        {
                            internal_symbol.kind.to_lowercase().contains("function")
                        } else {
                            false
                        }
                    } else {
                        true
                    }
                }
                GotoDefinitionResponse::Array(locs) => {
                    // Handle each location sequentially instead of using any()
                    let mut is_external_or_function = false;
                    for loc in locs {
                        let is_external = !original_symbol
                            .range
                            .contains(FilePosition::from(loc.clone()));
                        if is_external {
                            is_external_or_function = true;
                            break;
                        }
                        // Check if internal symbol is a function
                        if let Ok(internal_symbol) = self
                            .get_symbol_from_position(
                                &uri_to_relative_path_string(&loc.uri),
                                &loc.range.start.into(),
                            )
                            .await
                        {
                            if internal_symbol.kind.to_lowercase().contains("function") {
                                is_external_or_function = true;
                                break;
                            }
                        }
                    }
                    is_external_or_function
                }
                GotoDefinitionResponse::Link(links) => {
                    // Handle each link sequentially instead of using any()
                    let mut is_external_or_function = false;
                    for link in links {
                        let is_external = !original_symbol
                            .range
                            .contains(FilePosition::from(link.clone()));
                        if is_external {
                            is_external_or_function = true;
                            break;
                        }
                        // Check if internal symbol is a function
                        if let Ok(internal_symbol) = self
                            .get_symbol_from_position(
                                &uri_to_relative_path_string(&link.target_uri),
                                &link.target_range.start.into(),
                            )
                            .await
                        {
                            if internal_symbol.kind.to_lowercase().contains("function") {
                                is_external_or_function = true;
                                break;
                            }
                        }
                    }
                    is_external_or_function
                }
            };

            if has_external_definitions {
                // Found external definitions, return them
                return Ok(vec![definition]);
            }

            // All definitions are internal, need to look deeper
            let mut final_definitions = Vec::new();

            // For each internal definition location
            let locations = match &definition {
                GotoDefinitionResponse::Scalar(loc) => vec![loc.clone()],
                GotoDefinitionResponse::Array(locs) => locs.clone(),
                GotoDefinitionResponse::Link(links) => links
                    .iter()
                    .map(|l| Location::new(l.target_uri.clone(), l.target_range))
                    .collect(),
            };

            for location in locations {
                // Get the symbol at this definition
                let def_position = Position {
                    line: location.range.start.line,
                    character: location.range.start.character,
                };

                let internal_symbol = match self
                    .get_symbol_from_position(
                        &uri_to_relative_path_string(&location.uri),
                        &def_position,
                    )
                    .await
                {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                // Get all references within this symbol (with full_scan)
                let internal_references = match self
                    .ast_grep
                    .get_references_contained_in_symbol(
                        &uri_to_relative_path_string(&location.uri),
                        &internal_symbol,
                        true,
                    )
                    .await
                {
                    Ok(refs) => refs,
                    Err(_) => continue,
                };

                // For each reference, recursively resolve its definition chain
                for reference in internal_references {
                    let nested_definitions = self
                        .resolve_definition_chain(
                            &uri_to_relative_path_string(&location.uri),
                            original_symbol,
                            &reference,
                            client,
                        )
                        .await?;

                    final_definitions.extend(nested_definitions);
                }
            }

            if final_definitions.is_empty() {
                // If we found no external definitions through the chain, remove this branch
                Ok(vec![])
            } else {
                // Combine all locations from different GotoDefinitionResponses into a single Array variant
                let mut all_locations: Vec<Location> = final_definitions
                    .into_iter()
                    .flat_map(|def| match def {
                        GotoDefinitionResponse::Scalar(loc) => vec![loc],
                        GotoDefinitionResponse::Array(locs) => locs,
                        GotoDefinitionResponse::Link(links) => links
                            .into_iter()
                            .map(|link| Location::new(link.target_uri, link.target_range))
                            .collect(),
                    })
                    .collect();

                // Sort locations by file path, then line, then character
                all_locations.sort_by(|a, b| {
                    let path_a = uri_to_relative_path_string(&a.uri);
                    let path_b = uri_to_relative_path_string(&b.uri);
                    path_a
                        .cmp(&path_b)
                        .then(a.range.start.line.cmp(&b.range.start.line))
                        .then(a.range.start.character.cmp(&b.range.start.character))
                });

                Ok(vec![GotoDefinitionResponse::Array(all_locations)])
            }
        })
    }

    pub async fn find_referenced_symbols(
        &self,
        file_path: &str,
        position: Position,
    ) -> Result<Vec<(AstGrepMatch, GotoDefinitionResponse)>, LspManagerError> {
        let workspace_files = self.list_files().await.map_err(|e| {
            LspManagerError::InternalError(format!("Workspace file retrieval failed: {}", e))
        })?;

        if !workspace_files.iter().any(|f| f == file_path) {
            return Err(LspManagerError::FileNotFound(file_path.to_string()));
        }

        // First we find all the positions we need to find the definition of
        let symbol = match self
            .ast_grep
            .get_symbol_from_position(file_path, &position)
            .await
        {
            Ok(symbol) => symbol,
            Err(e) => {
                return Err(LspManagerError::InternalError(format!(
                    "Failed to find referenced symbols, {}",
                    e
                )));
            }
        };
        let references_to_symbols = match self
            .ast_grep
            .get_references_contained_in_symbol(file_path, &symbol, false)
            .await
        {
            Ok(referenced_symbols) => referenced_symbols,
            Err(e) => {
                return Err(LspManagerError::InternalError(format!(
                    "Failed to find referenced symbols, {}",
                    e
                )));
            }
        };

        let full_path = get_mount_dir().join(&file_path);
        let full_path_str = full_path.to_str().unwrap_or_default();
        let lsp_type = detect_language(full_path_str).map_err(|e| {
            LspManagerError::InternalError(format!("Language detection failed: {}", e))
        })?;
        let client = self
            .get_client(lsp_type)
            .ok_or(LspManagerError::LspClientNotFound(lsp_type))?;
        let mut locked_client = client.lock().await;
        let mut definitions = Vec::new();

        for ast_match in references_to_symbols {
            let def_chain = self
                .resolve_definition_chain(file_path, &symbol, &ast_match, &mut *locked_client)
                .await?;

            // Only include definitions that were found and led to external symbols
            if !def_chain.is_empty() {
                for def in def_chain {
                    definitions.push((ast_match.clone(), def));
                }
            }
        }

        Ok(definitions)
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

    pub async fn get_file_identifiers(
        &self,
        file_path: &str,
    ) -> Result<Vec<Identifier>, LspManagerError> {
        let full_path = get_mount_dir().join(&file_path);
        let workspace_files = self.list_files().await.map_err(|e| {
            LspManagerError::InternalError(format!("Workspace file retrieval failed: {}", e))
        })?;
        if !workspace_files.contains(&file_path.to_string()) {
            return Err(LspManagerError::FileNotFound(file_path.to_string()));
        }
        let full_path_str = full_path.to_str().unwrap_or_default();
        let ast_grep_result = self
            .ast_grep
            .get_file_identifiers(full_path_str)
            .await
            .map_err(|e| {
                LspManagerError::InternalError(format!("Symbol retrieval failed: {}", e))
            })?;
        Ok(ast_grep_result.into_iter().map(|s| s.into()).collect())
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
