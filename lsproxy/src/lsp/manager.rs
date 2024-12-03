use crate::api_types::{self, get_mount_dir, FileSymbolSubgraph, SupportedLanguages, Symbol};
use crate::ast_grep::client::AstGrepClient;
use crate::ast_grep::types::{AstGrepPatternMatch, AstGrepRuleMatch};
use crate::lsp::client::LspClient;
use crate::lsp::languages::{
    ClangdClient, JdtlsClient, JediClient, RustAnalyzerClient, TypeScriptLanguageClient,
};
use crate::utils::file_utils::{
    absolute_path_to_relative_path_string, search_files, uri_to_relative_path_string,
};
use crate::utils::workspace_documents::{
    WorkspaceDocuments, C_AND_CPP_EXTENSIONS, C_AND_CPP_FILE_PATTERNS, DEFAULT_EXCLUDE_PATTERNS,
    JAVA_EXTENSIONS, JAVA_FILE_PATTERNS, PYTHON_EXTENSIONS, PYTHON_FILE_PATTERNS, RUST_EXTENSIONS,
    RUST_FILE_PATTERNS, TYPESCRIPT_EXTENSIONS, TYPESCRIPT_FILE_PATTERNS,
};
use log::{debug, error, warn};
use lsp_types::{GotoDefinitionResponse, Location, Position, Range};
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, DebouncedEvent};
use std::collections::{HashMap, HashSet};
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
        ] {
            let patterns = match lsp {
                SupportedLanguages::Python => PYTHON_FILE_PATTERNS
                    .iter()
                    .map(|&s| s.to_string())
                    .collect(),
                SupportedLanguages::TypeScriptJavaScript => TYPESCRIPT_FILE_PATTERNS
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
            };
            if search_files(
                Path::new(root_path),
                patterns,
                DEFAULT_EXCLUDE_PATTERNS
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
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
        relative_file_path: &str,
    ) -> Result<Vec<AstGrepRuleMatch>, LspManagerError> {
        let workspace_files = self.list_files().await?;
        if !workspace_files.iter().any(|f| f == relative_file_path) {
            return Err(LspManagerError::FileNotFound(
                relative_file_path.to_string(),
            ));
        }
        let full_path = get_mount_dir().join(&relative_file_path);
        let full_path_str = full_path.to_str().unwrap_or_default();
        let ast_grep_result = self
            .ast_grep
            .get_file_symbols(full_path_str)
            .await
            .map_err(|e| LspManagerError::InternalError(format!("Symbol retrieval failed: {}", e)));
        ast_grep_result
    }

    /// Finds all references to imported symbols in a given file
    ///
    /// # Arguments
    /// * `relative_file_path` - The relative path to the file to analyze
    ///
    /// # Returns
    /// * `Result<Vec<AstGrepPatternMatch>, LspManagerError>` - A vector of pattern matches representing references to imported symbols
    ///
    /// # Errors
    /// * `LspManagerError::FileNotFound` if the file does not exist in the workspace
    /// * `LspManagerError::InternalError` if there is an error retrieving workspace files, symbols or references
    async fn references_to_imports_in_file(
        &self,
        relative_file_path: &str,
    ) -> Result<Vec<AstGrepPatternMatch>, LspManagerError> {
        let workspace_files = self.list_files().await.map_err(|e| {
            LspManagerError::InternalError(format!("Workspace file retrieval failed: {}", e))
        })?;
        if !workspace_files.iter().any(|f| f == relative_file_path) {
            return Err(LspManagerError::FileNotFound(relative_file_path.to_string()).into());
        }
        debug!("Getting imports for file: {}", relative_file_path);
        let full_file_path = get_mount_dir().join(&relative_file_path);
        let full_file_path_str = full_file_path.to_str().unwrap_or_default();
        let import_matches = self
            .ast_grep
            .get_file_imports(full_file_path_str)
            .await
            .map_err(|e| {
                LspManagerError::InternalError(format!("Symbol retrieval failed: {}", e))
            })?;
        debug!("Found {} imports", import_matches.len());
        self.ast_grep
            .get_references_to_imports(&import_matches)
            .await
            .map_err(|e| {
                LspManagerError::InternalError(format!("Reference retrieval failed: {}", e))
            })
    }

    /// Builds a graph of symbols and their relationships for a given file
    ///
    /// # Arguments
    /// * `relative_file_path` - The relative path to the file to analyze
    ///
    /// # Returns
    /// * `Result<FileSymbolSubgraph, LspManagerError>` - A graph containing:
    ///   - The symbols defined in the file
    ///   - Symbols from other files that reference these symbols
    ///   - Symbols from other files that are referenced by these symbols
    ///
    /// # Errors
    /// * `LspManagerError::FileNotFound` if the file does not exist in the workspace
    /// * `LspManagerError::InternalError` if there is an error retrieving workspace files, symbols or references
    pub async fn file_symbol_subgraph(
        &self,
        relative_file_path: &str,
    ) -> Result<FileSymbolSubgraph, LspManagerError> {
        let workspace_files = self.list_files().await.map_err(|e| {
            LspManagerError::InternalError(format!("Workspace file retrieval failed: {}", e))
        })?;
        if !workspace_files.iter().any(|f| f == relative_file_path) {
            return Err(LspManagerError::FileNotFound(relative_file_path.to_string()).into());
        }

        // Get all symbol definitions in the target file
        let file_symbol_matches = self
            .definitions_in_file_ast_grep(relative_file_path)
            .await?;
        let file_symbols: Vec<Symbol> = file_symbol_matches
            .into_iter()
            .map(|s| Symbol::from(s))
            .collect();

        if file_symbols.is_empty() {
            return Ok(FileSymbolSubgraph {
                symbols: vec![],
                referencing_symbols: vec![],
                referenced_symbols: vec![],
            });
        }
        // Find symbols in other files that reference our file's symbols
        let referencing_symbols = self.find_referencing_symbols(&file_symbols).await?;

        // Get references to imported symbols and their definitions
        let (references_to_imports, definitions_of_references_to_imports) = self
            .find_definitions_of_imported_referenced_symbols(relative_file_path)
            .await?;

        // For each symbol in our file, find which imported symbols it references
        let mut referenced_symbols = vec![];
        for symbol in &file_symbols {
            // Find references that are enclosed within this symbol's range
            let enclosed_references: Vec<(usize, &AstGrepPatternMatch)> = references_to_imports
                .iter()
                .enumerate()
                .filter(|(_, r)| {
                    symbol.range.contains(api_types::Position {
                        line: r.range.start.line as u32,
                        character: r.range.start.column as u32,
                    })
                })
                .collect();

            // Get the definitions for those enclosed references
            let referenced_symbols_for_defined_symbol: Vec<Symbol> = enclosed_references
                .iter()
                .filter_map(|(i, _)| definitions_of_references_to_imports[*i].clone())
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();
            referenced_symbols.push(referenced_symbols_for_defined_symbol);
        }

        Ok(FileSymbolSubgraph {
            symbols: file_symbols,
            referencing_symbols: referencing_symbols,
            referenced_symbols: referenced_symbols,
        })
    }

    /// Finds definitions of symbols that are imported then referenced in a file.
    ///
    /// # Arguments
    /// * `relative_file_path` - The relative path to the file to analyze
    ///
    /// # Returns
    /// A tuple containing:
    /// * A vector of AstGrepPatternMatch representing references to imports found in the file
    /// * A vector of optional Symbols representing the definitions of those referenced imports
    ///
    /// # Errors
    /// Returns LspManagerError if:
    /// * File references cannot be retrieved
    /// * Definition lookups fail
    /// * AST grep analysis fails
    async fn find_definitions_of_imported_referenced_symbols(
        &self,
        relative_file_path: &str,
    ) -> Result<(Vec<AstGrepPatternMatch>, Vec<Option<Symbol>>), LspManagerError> {
        debug!(
            "Finding definitions of imported referenced symbols in file: {}",
            relative_file_path
        );
        let references_to_imports = self
            .references_to_imports_in_file(relative_file_path)
            .await?;
        debug!(
            "Found {} references to imports",
            references_to_imports.len()
        );
        let definitions_responses_for_references_to_imports =
            futures::future::join_all(references_to_imports.iter().map(|r| async {
                let relative_file_path = r
                    .file
                    .as_str()
                    .strip_prefix(get_mount_dir().to_str().unwrap())
                    .and_then(|s| s.strip_prefix("/"));
                if relative_file_path.is_none() {
                    return Err(LspManagerError::InternalError(format!(
                        "Failed to convert URI: {}",
                        r.file
                    )));
                }
                self.find_definition(
                    relative_file_path.unwrap(),
                    Position {
                        line: r.range.start.line as u32,
                        character: r.range.start.column as u32,
                    },
                )
                .await
            }))
            .await;
        let definitions_of_references_to_imports: Vec<Option<Location>> =
            definitions_responses_for_references_to_imports
                .into_iter()
                .map(|r| match r {
                    Ok(d) => match d {
                        GotoDefinitionResponse::Scalar(l) => Some(l.clone()),
                        GotoDefinitionResponse::Array(l) => l.first().cloned(),
                        GotoDefinitionResponse::Link(_) => {
                            error!("Link definition response is not supported");
                            None
                        }
                    },
                    Err(e) => {
                        error!("Definition retrieval failed: {}", e);
                        None
                    }
                })
                .collect();
        let symbols_of_definitions_of_references_to_imports =
            futures::future::join_all(definitions_of_references_to_imports.iter().map(
                |d| async move {
                    if let Some(location) = d {
                        let relative_path = uri_to_relative_path_string(&location.uri).ok();
                        if relative_path.is_none() {
                            return None;
                        }
                        let matches = self
                            .definitions_in_file_ast_grep(&relative_path.unwrap())
                            .await
                            .unwrap();
                        let target_match = matches.iter().find(|m| {
                            m.range.start.line == location.range.start.line as usize
                                && m.range.start.column as u32 == location.range.start.character
                                && m.range.end.line == location.range.end.line as usize
                                && m.range.end.column as u32 == location.range.end.character
                        });
                        target_match.map(|m| Symbol::from(m.clone()))
                    } else {
                        None
                    }
                },
            ))
            .await;
        assert_eq!(
            references_to_imports.len(),
            symbols_of_definitions_of_references_to_imports.len()
        );
        Ok((
            references_to_imports,
            symbols_of_definitions_of_references_to_imports,
        ))
    }

    /// Finds all symbols that reference the given file symbols.
    ///
    /// For each symbol in `file_symbols`, finds all references to that symbol across the codebase.
    /// Then for each reference location, finds all symbols that enclose that reference position.
    ///
    /// # Arguments
    /// * `file_symbols` - Vector of symbols from a file to find references for
    ///
    /// # Returns
    /// * `Result<Vec<Vec<Symbol>>, LspManagerError>` - For each reference found, returns the vector of
    ///   symbols that enclose that reference position
    async fn find_referencing_symbols(
        &self,
        file_symbols: &Vec<Symbol>,
    ) -> Result<Vec<Vec<Symbol>>, LspManagerError> {
        if file_symbols.is_empty() {
            return Ok(vec![]);
        }
        let first_symbol = file_symbols.first();
        if first_symbol.is_none() {
            return Ok(vec![]);
        }
        let relative_file_path = first_symbol.unwrap().identifier_position.path.clone();
        let references_by_symbol: Vec<Result<Vec<Location>, LspManagerError>> =
            futures::future::join_all(file_symbols.iter().map(|s| {
                self.find_references(
                    &relative_file_path,
                    Position {
                        line: s.identifier_position.position.line,
                        character: s.identifier_position.position.character,
                    },
                )
            }))
            .await
            .into_iter()
            .collect();
        let mut referencing_symbols: Vec<Vec<Symbol>> = vec![];
        for (i, reference_list) in references_by_symbol.into_iter().enumerate() {
            let locations = reference_list.unwrap_or(vec![]);
            let mut referencing_symbols_for_symbol = vec![];

            for location in locations {
                let relative_path = uri_to_relative_path_string(&location.uri).ok();
                if relative_path.is_none() {
                    continue;
                }
                let mut symbols = self
                    .find_symbols_enclosing_position(&relative_path.unwrap(), location.range.start)
                    .await?;
                symbols.retain(|s| s != &file_symbols[i]);
                referencing_symbols_for_symbol.extend(symbols);
            }
            referencing_symbols.push(referencing_symbols_for_symbol);
        }
        Ok(referencing_symbols)
    }

    /// Finds all symbols that enclose a given position in a file.
    ///
    /// A symbol "encloses" a position if the position falls within the symbol's range.
    /// For example, a function symbol encloses all positions between its opening and closing braces.
    ///
    /// # Arguments
    /// * `file_path` - The relative path to the file to search in
    /// * `position` - The position to find enclosing symbols for
    ///
    /// # Returns
    /// * `Result<Vec<Symbol>, LspManagerError>` - Vector of symbols that enclose the given position
    ///
    /// # Errors
    /// * `LspManagerError` if there is an error retrieving symbols from the file
    async fn find_symbols_enclosing_position(
        &self,
        file_path: &str,
        position: Position,
    ) -> Result<Vec<Symbol>, LspManagerError> {
        let file_symbols = self.definitions_in_file_ast_grep(file_path).await?;
        let mut symbols: Vec<Symbol> = file_symbols.into_iter().map(|s| Symbol::from(s)).collect();
        symbols.retain(|s| {
            (s.range.start.line < position.line
                || (s.range.start.line == position.line
                    && s.range.start.character <= position.character))
                && (s.range.end.line > position.line
                    || (s.range.end.line == position.line
                        && s.range.end.character >= position.character))
        });
        Ok(symbols)
    }

    pub async fn find_definition(
        &self,
        relative_file_path: &str,
        position: Position,
    ) -> Result<GotoDefinitionResponse, LspManagerError> {
        let workspace_files = self.list_files().await.map_err(|e| {
            LspManagerError::InternalError(format!("Workspace file retrieval failed: {}", e))
        })?;
        if !workspace_files.iter().any(|f| f == relative_file_path) {
            return Err(LspManagerError::FileNotFound(relative_file_path.to_string()).into());
        }
        let full_path = get_mount_dir().join(&relative_file_path);
        let full_path_str = full_path.to_str().unwrap_or_default();
        let lsp_type = self.detect_language(full_path_str).map_err(|e| {
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
        let lsp_type = self.detect_language(full_path_str).map_err(|e| {
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
                    .filter_map(|f| absolute_path_to_relative_path_string(f).ok())
                    .collect::<Vec<String>>(),
            );
        }
        files.sort();
        Ok(files)
    }

    fn detect_language(
        &self,
        relative_file_path: &str,
    ) -> Result<SupportedLanguages, LspManagerError> {
        let path = PathBuf::from(relative_file_path);
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| LspManagerError::UnsupportedFileType(relative_file_path.to_string()))?;

        match extension {
            ext if PYTHON_EXTENSIONS.contains(&ext) => Ok(SupportedLanguages::Python),
            ext if TYPESCRIPT_EXTENSIONS.contains(&ext) => {
                Ok(SupportedLanguages::TypeScriptJavaScript)
            }
            ext if RUST_EXTENSIONS.contains(&ext) => Ok(SupportedLanguages::Rust),
            ext if C_AND_CPP_EXTENSIONS.contains(&ext) => Ok(SupportedLanguages::CPP),
            ext if JAVA_EXTENSIONS.contains(&ext) => Ok(SupportedLanguages::Java),
            _ => Err(LspManagerError::UnsupportedFileType(
                relative_file_path.to_string(),
            )),
        }
    }

    pub async fn read_source_code(
        &self,
        relative_file_path: &str,
        range: Option<Range>,
    ) -> Result<String, LspManagerError> {
        let client = self
            .get_client(self.detect_language(relative_file_path)?)
            .ok_or(LspManagerError::LspClientNotFound(
                self.detect_language(relative_file_path)?,
            ))?;
        let full_path = get_mount_dir().join(&relative_file_path);
        let mut locked_client = client.lock().await;
        locked_client
            .get_workspace_documents()
            .read_text_document(&full_path, range)
            .await
            .map_err(|e| {
                LspManagerError::InternalError(format!("Source code retrieval failed: {}", e))
            })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_types::{FilePosition, FileRange, Position, Symbol, SymbolResponse};
    use crate::test_utils::{
        c_sample_path, cpp_sample_path, java_sample_path, js_sample_path, python_sample_path,
        rust_sample_path, typescript_sample_path, TestContext,
    };
    use lsp_types::{Range, Url};

    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_start_manager_python() -> Result<(), Box<dyn std::error::Error>> {
        TestContext::setup(&python_sample_path(), true).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_workspace_files_python() -> Result<(), Box<dyn std::error::Error>> {
        let context = TestContext::setup(&python_sample_path(), true).await?;
        let manager = context
            .manager
            .as_ref()
            .ok_or("Manager is not initialized")?;

        let mut result = manager.list_files().await?;
        let mut expected = vec!["graph.py", "main.py", "search.py", "__init__.py"];

        assert_eq!(result.sort(), expected.sort());
        Ok(())
    }

    #[tokio::test]
    async fn test_file_symbols_python() -> Result<(), Box<dyn std::error::Error>> {
        let context = TestContext::setup(&python_sample_path(), true).await?;
        let manager = context
            .manager
            .as_ref()
            .ok_or("Manager is not initialized")?;

        let file_path = "main.py";
        let file_symbols = manager.definitions_in_file_ast_grep(file_path).await?;

        let symbol_response: SymbolResponse =
            file_symbols.into_iter().map(|s| Symbol::from(s)).collect();

        let expected = vec![
            Symbol {
                name: String::from("graph"),
                kind: String::from("variable"),
                identifier_position: FilePosition {
                    path: String::from("main.py"),
                    position: Position {
                        line: 5,
                        character: 0,
                    },
                },
                range: FileRange {
                    path: String::from("main.py"),
                    start: Position {
                        line: 5,
                        character: 0,
                    },
                    end: Position {
                        line: 5,
                        character: 20,
                    },
                },
            },
            Symbol {
                name: String::from("result"),
                kind: String::from("variable"),
                identifier_position: FilePosition {
                    path: String::from("main.py"),
                    position: Position {
                        line: 6,
                        character: 0,
                    },
                },
                range: FileRange {
                    path: String::from("main.py"),
                    start: Position {
                        line: 6,
                        character: 0,
                    },
                    end: Position {
                        line: 6,
                        character: 51,
                    },
                },
            },
            Symbol {
                name: String::from("cost"),
                kind: String::from("variable"),
                identifier_position: FilePosition {
                    path: String::from("main.py"),
                    position: Position {
                        line: 6,
                        character: 8,
                    },
                },
                range: FileRange {
                    path: String::from("main.py"),
                    start: Position {
                        line: 6,
                        character: 0,
                    },
                    end: Position {
                        line: 6,
                        character: 51,
                    },
                },
            },
        ];
        assert_eq!(symbol_response, expected);
        Ok(())
    }

    #[tokio::test]
    async fn test_file_symbols_python_decorators() -> Result<(), Box<dyn std::error::Error>> {
        let context = TestContext::setup(&python_sample_path(), true).await?;
        let manager = context
            .manager
            .as_ref()
            .ok_or("Manager is not initialized")?;

        let file_path = "graph.py";
        let file_symbols = manager.definitions_in_file_ast_grep(file_path).await?;

        let symbol_response: SymbolResponse =
            file_symbols.into_iter().map(|s| Symbol::from(s)).collect();

        let expected = vec![
            Symbol {
                name: String::from("AStarGraph"),
                kind: String::from("class"),
                identifier_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 1,
                        character: 6,
                    },
                },
                range: FileRange {
                    path: String::from("graph.py"),
                    start: Position {
                        line: 1,
                        character: 0,
                    },
                    end: Position {
                        line: 60,
                        character: 40,
                    },
                },
            },
            Symbol {
                name: String::from("__init__"),
                kind: String::from("function"),
                identifier_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 4,
                        character: 8,
                    },
                },
                range: FileRange {
                    path: String::from("graph.py"),
                    start: Position {
                        line: 4,
                        character: 0,
                    },
                    end: Position {
                        line: 21,
                        character: 9,
                    },
                },
            },
            Symbol {
                name: String::from("barriers"),
                kind: String::from("function"),
                identifier_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 24,
                        character: 8,
                    },
                },
                range: FileRange {
                    path: String::from("graph.py"),
                    start: Position {
                        line: 23,
                        character: 0,
                    },
                    end: Position {
                        line: 25,
                        character: 28,
                    },
                },
            },
            Symbol {
                name: String::from("heuristic"),
                kind: String::from("function"),
                identifier_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 27,
                        character: 8,
                    },
                },
                range: FileRange {
                    path: String::from("graph.py"),
                    start: Position {
                        line: 27,
                        character: 0,
                    },
                    end: Position {
                        line: 34,
                        character: 57,
                    },
                },
            },
            Symbol {
                name: String::from("get_vertex_neighbours"),
                kind: String::from("function"),
                identifier_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 36,
                        character: 8,
                    },
                },
                range: FileRange {
                    path: String::from("graph.py"),
                    start: Position {
                        line: 36,
                        character: 0,
                    },
                    end: Position {
                        line: 54,
                        character: 16,
                    },
                },
            },
            Symbol {
                name: String::from("move_cost"),
                kind: String::from("function"),
                identifier_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 56,
                        character: 8,
                    },
                },
                range: FileRange {
                    path: String::from("graph.py"),
                    start: Position {
                        line: 56,
                        character: 0,
                    },
                    end: Position {
                        line: 60,
                        character: 40,
                    },
                },
            },
        ];
        assert_eq!(symbol_response, expected);
        Ok(())
    }

    #[tokio::test]
    async fn test_file_symbols_cpp() -> Result<(), Box<dyn std::error::Error>> {
        let context = TestContext::setup(&cpp_sample_path(), true).await?;
        let manager = context
            .manager
            .as_ref()
            .ok_or("Manager is not initialized")?;

        let file_path = "cpp_classes/astar.cpp";
        let file_symbols = manager.definitions_in_file_ast_grep(file_path).await?;
        let symbol_response: SymbolResponse =
            file_symbols.into_iter().map(|s| Symbol::from(s)).collect();

        let expected = vec![
            Symbol {
                name: String::from("aStar"),
                kind: String::from("class"),
                identifier_position: FilePosition {
                    path: String::from("cpp_classes/astar.cpp"),
                    position: Position {
                        line: 8,
                        character: 6,
                    },
                },
                range: FileRange {
                    path: String::from("cpp_classes/astar.cpp"),
                    start: Position {
                        line: 8,
                        character: 0,
                    },
                    end: Position {
                        line: 101,
                        character: 1,
                    },
                },
            },
            Symbol {
                name: String::from("aStar"),
                kind: String::from("function-definition"),
                identifier_position: FilePosition {
                    path: String::from("cpp_classes/astar.cpp"),
                    position: Position {
                        line: 10,
                        character: 4,
                    },
                },
                range: FileRange {
                    path: String::from("cpp_classes/astar.cpp"),
                    start: Position {
                        line: 10,
                        character: 0,
                    },
                    end: Position {
                        line: 15,
                        character: 5,
                    },
                },
            },
            Symbol {
                name: String::from("calcDist"),
                kind: String::from("function-definition"),
                identifier_position: FilePosition {
                    path: String::from("cpp_classes/astar.cpp"),
                    position: Position {
                        line: 17,
                        character: 8,
                    },
                },
                range: FileRange {
                    path: String::from("cpp_classes/astar.cpp"),
                    start: Position {
                        line: 17,
                        character: 0,
                    },
                    end: Position {
                        line: 21,
                        character: 5,
                    },
                },
            },
            Symbol {
                name: String::from("isValid"),
                kind: String::from("function-definition"),
                identifier_position: FilePosition {
                    path: String::from("cpp_classes/astar.cpp"),
                    position: Position {
                        line: 23,
                        character: 9,
                    },
                },
                range: FileRange {
                    path: String::from("cpp_classes/astar.cpp"),
                    start: Position {
                        line: 23,
                        character: 0,
                    },
                    end: Position {
                        line: 25,
                        character: 5,
                    },
                },
            },
            Symbol {
                name: String::from("existPoint"),
                kind: String::from("function-definition"),
                identifier_position: FilePosition {
                    path: String::from("cpp_classes/astar.cpp"),
                    position: Position {
                        line: 27,
                        character: 9,
                    },
                },
                range: FileRange {
                    path: String::from("cpp_classes/astar.cpp"),
                    start: Position {
                        line: 27,
                        character: 0,
                    },
                    end: Position {
                        line: 40,
                        character: 5,
                    },
                },
            },
            Symbol {
                name: String::from("fillOpen"),
                kind: String::from("function-definition"),
                identifier_position: FilePosition {
                    path: String::from("cpp_classes/astar.cpp"),
                    position: Position {
                        line: 42,
                        character: 9,
                    },
                },
                range: FileRange {
                    path: String::from("cpp_classes/astar.cpp"),
                    start: Position {
                        line: 42,
                        character: 0,
                    },
                    end: Position {
                        line: 65,
                        character: 5,
                    },
                },
            },
            Symbol {
                name: String::from("search"),
                kind: String::from("function-definition"),
                identifier_position: FilePosition {
                    path: String::from("cpp_classes/astar.cpp"),
                    position: Position {
                        line: 67,
                        character: 9,
                    },
                },
                range: FileRange {
                    path: String::from("cpp_classes/astar.cpp"),
                    start: Position {
                        line: 67,
                        character: 0,
                    },
                    end: Position {
                        line: 79,
                        character: 5,
                    },
                },
            },
            Symbol {
                name: String::from("path"),
                kind: String::from("function-definition"),
                identifier_position: FilePosition {
                    path: String::from("cpp_classes/astar.cpp"),
                    position: Position {
                        line: 81,
                        character: 8,
                    },
                },
                range: FileRange {
                    path: String::from("cpp_classes/astar.cpp"),
                    start: Position {
                        line: 81,
                        character: 0,
                    },
                    end: Position {
                        line: 95,
                        character: 5,
                    },
                },
            },
        ];

        assert_eq!(symbol_response, expected);
        Ok(())
    }

    #[tokio::test]
    async fn test_file_symbols_js() -> Result<(), Box<dyn std::error::Error>> {
        let context = TestContext::setup(&js_sample_path(), true).await?;
        let manager = context
            .manager
            .as_ref()
            .ok_or("Manager is not initialized")?;

        let file_path = "astar_search.js";
        let file_symbols = manager.definitions_in_file_ast_grep(file_path).await?;
        // TODO: include source code and update expected
        let mut symbol_response: SymbolResponse =
            file_symbols.into_iter().map(|s| Symbol::from(s)).collect();

        let mut expected = vec![
            Symbol {
                name: String::from("manhattan"),
                kind: String::from("function"),
                identifier_position: FilePosition {
                    path: String::from("astar_search.js"),
                    position: Position {
                        line: 0,
                        character: 9,
                    },
                },
                range: FileRange {
                    path: String::from("astar_search.js"),
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 2,
                        character: 1,
                    },
                },
            },
            Symbol {
                name: String::from("aStar"),
                kind: String::from("function"),
                identifier_position: FilePosition {
                    path: String::from("astar_search.js"),
                    position: Position {
                        line: 4,
                        character: 9,
                    },
                },
                range: FileRange {
                    path: String::from("astar_search.js"),
                    start: Position {
                        line: 4,
                        character: 0,
                    },
                    end: Position {
                        line: 58,
                        character: 1,
                    },
                },
            },
            Symbol {
                name: String::from("lambda"),
                kind: String::from("function"),
                identifier_position: FilePosition {
                    path: String::from("astar_search.js"),
                    position: Position {
                        line: 17,
                        character: 16,
                    },
                },
                range: FileRange {
                    path: String::from("astar_search.js"),
                    start: Position {
                        line: 17,
                        character: 0,
                    },
                    end: Position {
                        line: 26,
                        character: 9,
                    },
                },
            },
            Symbol {
                name: String::from("board"),
                kind: String::from("variable"),
                identifier_position: FilePosition {
                    path: String::from("astar_search.js"),
                    position: Position {
                        line: 60,
                        character: 6,
                    },
                },
                range: FileRange {
                    path: String::from("astar_search.js"),
                    start: Position {
                        line: 60,
                        character: 0,
                    },
                    end: Position {
                        line: 69,
                        character: 1,
                    },
                },
            },
        ];

        // sort symbols by name
        symbol_response.sort_by_key(|s| s.name.clone());
        expected.sort_by_key(|s| s.name.clone());
        assert_eq!(symbol_response, expected);
        Ok(())
    }

    #[tokio::test]
    async fn test_file_symbols_java() -> Result<(), Box<dyn std::error::Error>> {
        let context = TestContext::setup(&java_sample_path(), true).await?;
        let manager = context
            .manager
            .as_ref()
            .ok_or("Manager is not initialized")?;
        let file_path = "AStar.java";
        let file_symbols = manager.definitions_in_file_ast_grep(file_path).await?;
        let mut symbol_response: SymbolResponse =
            file_symbols.into_iter().map(|s| Symbol::from(s)).collect();

        let mut expected = vec![
            Symbol {
                name: String::from("AStar"),
                kind: String::from("class"),
                identifier_position: FilePosition {
                    path: String::from("AStar.java"),
                    position: Position {
                        line: 10,
                        character: 13,
                    },
                },
                range: FileRange {
                    path: String::from("AStar.java"),
                    start: Position {
                        line: 10,
                        character: 0,
                    },
                    end: Position {
                        line: 96,
                        character: 21,
                    },
                },
            },
            Symbol {
                name: String::from("findPathTo"),
                kind: String::from("method"),
                identifier_position: FilePosition {
                    path: String::from("AStar.java"),
                    position: Position {
                        line: 39,
                        character: 22,
                    },
                },
                range: FileRange {
                    path: String::from("AStar.java"),
                    start: Position {
                        line: 39,
                        character: 0,
                    },
                    end: Position {
                        line: 59,
                        character: 5,
                    },
                },
            },
            Symbol {
                name: String::from("addNeigborsToOpenList"),
                kind: String::from("method"),
                identifier_position: FilePosition {
                    path: String::from("AStar.java"),
                    position: Position {
                        line: 61,
                        character: 17,
                    },
                },
                range: FileRange {
                    path: String::from("AStar.java"),
                    start: Position {
                        line: 61,
                        character: 0,
                    },
                    end: Position {
                        line: 89,
                        character: 41,
                    },
                },
            },
            Symbol {
                name: String::from("distance"),
                kind: String::from("method"),
                identifier_position: FilePosition {
                    path: String::from("AStar.java"),
                    position: Position {
                        line: 93,
                        character: 55,
                    },
                },
                range: FileRange {
                    path: String::from("AStar.java"),
                    start: Position {
                        line: 93,
                        character: 0,
                    },
                    end: Position {
                        line: 95,
                        character: 41,
                    },
                },
            },
            Symbol {
                name: String::from("main"),
                kind: String::from("method"),
                identifier_position: FilePosition {
                    path: String::from("AStar.java"),
                    position: Position {
                        line: 98,
                        character: 59,
                    },
                },
                range: FileRange {
                    path: String::from("AStar.java"),
                    start: Position {
                        line: 98,
                        character: 0,
                    },
                    end: Position {
                        line: 136,
                        character: 5,
                    },
                },
            },
            Symbol {
                name: String::from("findNeighborInList"),
                kind: String::from("method"),
                identifier_position: FilePosition {
                    path: String::from("AStar.java"),
                    position: Position {
                        line: 138,
                        character: 20,
                    },
                },
                range: FileRange {
                    path: String::from("AStar.java"),
                    start: Position {
                        line: 138,
                        character: 0,
                    },
                    end: Position {
                        line: 140,
                        character: 5,
                    },
                },
            },
        ];

        // sort symbols by name
        symbol_response.sort_by_key(|s| s.name.clone());
        expected.sort_by_key(|s| s.name.clone());
        assert_eq!(symbol_response, expected);
        Ok(())
    }

    #[tokio::test]
    async fn test_file_symbols_rust() -> Result<(), Box<dyn std::error::Error>> {
        let context = TestContext::setup(&rust_sample_path(), true).await?;
        let manager = context
            .manager
            .as_ref()
            .ok_or("Manager is not initialized")?;
        let file_path = "src/map.rs";
        let file_symbols = manager.definitions_in_file_ast_grep(file_path).await?;
        let mut symbol_response: SymbolResponse =
            file_symbols.into_iter().map(|s| Symbol::from(s)).collect();

        let mut expected = vec![
            Symbol {
                name: String::from("Map"),
                kind: String::from("struct"),
                identifier_position: FilePosition {
                    path: String::from("src/map.rs"),
                    position: Position {
                        line: 0,
                        character: 11,
                    },
                },
                range: FileRange {
                    path: String::from("src/map.rs"),
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 4,
                        character: 1,
                    },
                },
            },
            Symbol {
                name: String::from("Map"),
                kind: String::from("implementation"),
                identifier_position: FilePosition {
                    path: String::from("src/map.rs"),
                    position: Position {
                        line: 6,
                        character: 5,
                    },
                },
                range: FileRange {
                    path: String::from("src/map.rs"),
                    start: Position {
                        line: 6,
                        character: 0,
                    },
                    end: Position {
                        line: 24,
                        character: 1,
                    },
                },
            },
            Symbol {
                name: String::from("get"),
                kind: String::from("function"),
                identifier_position: FilePosition {
                    path: String::from("src/map.rs"),
                    position: Position {
                        line: 21,
                        character: 11,
                    },
                },
                range: FileRange {
                    path: String::from("src/map.rs"),
                    start: Position {
                        line: 21,
                        character: 0,
                    },
                    end: Position {
                        line: 23,
                        character: 5,
                    },
                },
            },
            Symbol {
                name: String::from("new"),
                kind: String::from("function"),
                identifier_position: FilePosition {
                    path: String::from("src/map.rs"),
                    position: Position {
                        line: 7,
                        character: 11,
                    },
                },
                range: FileRange {
                    path: String::from("src/map.rs"),
                    start: Position {
                        line: 7,
                        character: 0,
                    },
                    end: Position {
                        line: 19,
                        character: 5,
                    },
                },
            },
        ];
        // sort symbols by name
        symbol_response.sort_by_key(|s| s.name.clone());
        expected.sort_by_key(|s| s.name.clone());
        assert_eq!(symbol_response, expected);
        Ok(())
    }

    #[tokio::test]
    async fn test_file_symbols_typescript() -> Result<(), Box<dyn std::error::Error>> {
        let context = TestContext::setup(&typescript_sample_path(), true).await?;
        let manager = context
            .manager
            .as_ref()
            .ok_or("Manager is not initialized")?;
        let file_path = "node.ts";
        let file_symbols = manager.definitions_in_file_ast_grep(file_path).await?;
        let mut symbol_response: SymbolResponse =
            file_symbols.into_iter().map(|s| Symbol::from(s)).collect();

        let mut expected = vec![
            Symbol {
                name: String::from("Node"),
                kind: String::from("class"),
                identifier_position: FilePosition {
                    path: String::from("node.ts"),
                    position: Position {
                        line: 0,
                        character: 13,
                    },
                },
                range: FileRange {
                    path: String::from("node.ts"),
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 14,
                        character: 1,
                    },
                },
            },
            Symbol {
                name: String::from("constructor"),
                kind: String::from("method"),
                identifier_position: FilePosition {
                    path: String::from("node.ts"),
                    position: Position {
                        line: 1,
                        character: 4,
                    },
                },
                range: FileRange {
                    path: String::from("node.ts"),
                    start: Position {
                        line: 1,
                        character: 0,
                    },
                    end: Position {
                        line: 7,
                        character: 8,
                    },
                },
            },
            Symbol {
                name: String::from("f"),
                kind: String::from("method"),
                identifier_position: FilePosition {
                    path: String::from("node.ts"),
                    position: Position {
                        line: 10,
                        character: 4,
                    },
                },
                range: FileRange {
                    path: String::from("node.ts"),
                    start: Position {
                        line: 10,
                        character: 0,
                    },
                    end: Position {
                        line: 10,
                        character: 37,
                    },
                },
            },
            Symbol {
                name: String::from("toString"),
                kind: String::from("method"),
                identifier_position: FilePosition {
                    path: String::from("node.ts"),
                    position: Position {
                        line: 13,
                        character: 4,
                    },
                },
                range: FileRange {
                    path: String::from("node.ts"),
                    start: Position {
                        line: 13,
                        character: 0,
                    },
                    end: Position {
                        line: 13,
                        character: 57,
                    },
                },
            },
        ];
        // sort symbols by name
        symbol_response.sort_by_key(|s| s.name.clone());
        expected.sort_by_key(|s| s.name.clone());
        assert_eq!(symbol_response, expected);
        Ok(())
    }

    #[tokio::test]
    async fn test_file_symbols_tsx() -> Result<(), Box<dyn std::error::Error>> {
        let context = TestContext::setup(&typescript_sample_path(), true).await?;
        let manager = context
            .manager
            .as_ref()
            .ok_or("Manager is not initialized")?;
        let file_path = "PathfinderDisplay.tsx";
        let file_symbols = manager.definitions_in_file_ast_grep(file_path).await?;
        let mut symbol_response: SymbolResponse =
            file_symbols.into_iter().map(|s| Symbol::from(s)).collect();

        let mut expected = vec![
            Symbol {
                name: String::from("PathfinderDisplay"),
                kind: String::from("function"),
                identifier_position: FilePosition {
                    path: String::from("PathfinderDisplay.tsx"),
                    position: Position {
                        line: 12,
                        character: 13,
                    },
                },
                range: FileRange {
                    path: String::from("PathfinderDisplay.tsx"),
                    start: Position {
                        line: 12,
                        character: 0,
                    },
                    end: Position {
                        line: 125,
                        character: 1,
                    },
                },
            },
            Symbol {
                name: String::from("PathfinderDisplayProps"),
                kind: String::from("interface"),
                identifier_position: FilePosition {
                    path: String::from("PathfinderDisplay.tsx"),
                    position: Position {
                        line: 5,
                        character: 10,
                    },
                },
                range: FileRange {
                    path: String::from("PathfinderDisplay.tsx"),
                    start: Position {
                        line: 5,
                        character: 0,
                    },
                    end: Position {
                        line: 10,
                        character: 1,
                    },
                },
            },
            Symbol {
                name: String::from("findPath"),
                kind: String::from("function"),
                identifier_position: FilePosition {
                    path: String::from("PathfinderDisplay.tsx"),
                    position: Position {
                        line: 32,
                        character: 10,
                    },
                },
                range: FileRange {
                    path: String::from("PathfinderDisplay.tsx"),
                    start: Position {
                        line: 32,
                        character: 0,
                    },
                    end: Position {
                        line: 38,
                        character: 5,
                    },
                },
            },
            Symbol {
                name: String::from("getCellColor"),
                kind: String::from("function"),
                identifier_position: FilePosition {
                    path: String::from("PathfinderDisplay.tsx"),
                    position: Position {
                        line: 52,
                        character: 10,
                    },
                },
                range: FileRange {
                    path: String::from("PathfinderDisplay.tsx"),
                    start: Position {
                        line: 52,
                        character: 0,
                    },
                    end: Position {
                        line: 61,
                        character: 5,
                    },
                },
            },
            Symbol {
                name: String::from("toggleCell"),
                kind: String::from("function"),
                identifier_position: FilePosition {
                    path: String::from("PathfinderDisplay.tsx"),
                    position: Position {
                        line: 63,
                        character: 10,
                    },
                },
                range: FileRange {
                    path: String::from("PathfinderDisplay.tsx"),
                    start: Position {
                        line: 63,
                        character: 0,
                    },
                    end: Position {
                        line: 71,
                        character: 5,
                    },
                },
            },
        ];
        // sort symbols by name
        symbol_response.sort_by_key(|s| s.name.clone());
        expected.sort_by_key(|s| s.name.clone());
        assert_eq!(symbol_response, expected);
        Ok(())
    }

    #[tokio::test]
    async fn test_references_c() -> Result<(), Box<dyn std::error::Error>> {
        let context = TestContext::setup(&c_sample_path(), true).await?;
        let manager = context
            .manager
            .as_ref()
            .ok_or("Manager is not initialized")?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        let references = manager
            .find_references(
                "map.c",
                lsp_types::Position {
                    line: 30,
                    character: 5,
                },
            )
            .await?;

        let expected = vec![
            Location {
                uri: Url::parse("file:///mnt/lsproxy_root/sample_project/c/map.c").unwrap(),
                range: lsp_types::Range {
                    start: lsp_types::Position {
                        line: 30,
                        character: 5,
                    },
                    end: lsp_types::Position {
                        line: 30,
                        character: 14,
                    },
                },
            },
            Location {
                uri: Url::parse("file:///mnt/lsproxy_root/sample_project/c/main.c").unwrap(),
                range: Range {
                    start: lsp_types::Position {
                        line: 15,
                        character: 8,
                    },
                    end: lsp_types::Position {
                        line: 15,
                        character: 17,
                    },
                },
            },
            Location {
                uri: Url::parse("file:///mnt/lsproxy_root/sample_project/c/map.h").unwrap(),
                range: Range {
                    start: lsp_types::Position {
                        line: 11,
                        character: 5,
                    },
                    end: lsp_types::Position {
                        line: 11,
                        character: 14,
                    },
                },
            },
        ];

        // Sort locations before comparing
        let mut actual_locations = references;
        let mut expected_locations = expected;

        actual_locations.sort_by(|a, b| a.uri.path().cmp(&b.uri.path()));
        expected_locations.sort_by(|a, b| a.uri.path().cmp(&b.uri.path()));

        assert_eq!(actual_locations, expected_locations);
        Ok(())
    }

    #[tokio::test]
    async fn test_references_python() -> Result<(), Box<dyn std::error::Error>> {
        let context = TestContext::setup(&python_sample_path(), true).await?;
        let manager = context
            .manager
            .as_ref()
            .ok_or("Manager is not initialized")?;
        let file_path = "graph.py";

        let references = manager
            .find_references(
                file_path,
                lsp_types::Position {
                    line: 1,
                    character: 6,
                },
            )
            .await?;

        let expected = vec![
            Location {
                uri: Url::parse("file:///mnt/lsproxy_root/sample_project/python/graph.py").unwrap(),
                range: Range {
                    start: lsp_types::Position {
                        line: 1,
                        character: 6,
                    },
                    end: lsp_types::Position {
                        line: 1,
                        character: 16,
                    },
                },
            },
            Location {
                uri: Url::parse("file:///mnt/lsproxy_root/sample_project/python/main.py").unwrap(),
                range: Range {
                    start: lsp_types::Position {
                        line: 1,
                        character: 18,
                    },
                    end: lsp_types::Position {
                        line: 1,
                        character: 28,
                    },
                },
            },
            Location {
                uri: Url::parse("file:///mnt/lsproxy_root/sample_project/python/main.py").unwrap(),
                range: Range {
                    start: lsp_types::Position {
                        line: 5,
                        character: 8,
                    },
                    end: lsp_types::Position {
                        line: 5,
                        character: 18,
                    },
                },
            },
        ];
        assert_eq!(references, expected);

        Ok(())
    }

    #[tokio::test]
    async fn test_definition_python() -> Result<(), Box<dyn std::error::Error>> {
        let context = TestContext::setup(&python_sample_path(), true).await?;
        let manager = context
            .manager
            .as_ref()
            .ok_or("Manager is not initialized")?;
        let def_response = manager
            .find_definition(
                "main.py",
                lsp_types::Position {
                    line: 1,
                    character: 18,
                },
            )
            .await?;

        let definitions = match def_response {
            GotoDefinitionResponse::Scalar(location) => vec![location],
            GotoDefinitionResponse::Array(locations) => locations,
            GotoDefinitionResponse::Link(_links) => Vec::new(),
        };

        assert_eq!(
            definitions,
            vec![Location {
                uri: Url::parse("file:///mnt/lsproxy_root/sample_project/python/graph.py").unwrap(),
                range: Range {
                    start: lsp_types::Position {
                        line: 1,
                        character: 6,
                    },
                    end: lsp_types::Position {
                        line: 1,
                        character: 16,
                    },
                },
            }]
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_start_manager_js() -> Result<(), Box<dyn std::error::Error>> {
        TestContext::setup(&js_sample_path(), true).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_workspace_files_js() -> Result<(), Box<dyn std::error::Error>> {
        let context = TestContext::setup(&js_sample_path(), true).await?;

        let manager = context
            .manager
            .as_ref()
            .ok_or("Manager is not initialized")?;
        let files = manager.list_files().await?;

        assert_eq!(files, vec!["astar_search.js"]);
        Ok(())
    }

    #[tokio::test]
    async fn test_references_java() -> Result<(), Box<dyn std::error::Error>> {
        let context = TestContext::setup(&java_sample_path(), true).await?;
        let manager = context
            .manager
            .as_ref()
            .ok_or("Manager is not initialized")?;
        let file_path = "AStar.java";
        let references = manager
            .find_references(
                file_path,
                lsp_types::Position {
                    line: 10,
                    character: 13,
                },
            )
            .await?;

        let expected = vec![
            Location {
                uri: Url::parse("file:///mnt/lsproxy_root/sample_project/java/AStar.java").unwrap(),
                range: Range {
                    start: lsp_types::Position {
                        line: 10,
                        character: 13,
                    },
                    end: lsp_types::Position {
                        line: 10,
                        character: 18,
                    },
                },
            },
            Location {
                uri: Url::parse("file:///mnt/lsproxy_root/sample_project/java/AStar.java").unwrap(),
                range: Range {
                    start: lsp_types::Position {
                        line: 111,
                        character: 8,
                    },
                    end: lsp_types::Position {
                        line: 111,
                        character: 13,
                    },
                },
            },
            Location {
                uri: Url::parse("file:///mnt/lsproxy_root/sample_project/java/AStar.java").unwrap(),
                range: Range {
                    start: lsp_types::Position {
                        line: 111,
                        character: 23,
                    },
                    end: lsp_types::Position {
                        line: 111,
                        character: 28,
                    },
                },
            },
        ];
        assert_eq!(references, expected);
        Ok(())
    }

    #[tokio::test]
    async fn test_definition_java() -> Result<(), Box<dyn std::error::Error>> {
        let context = TestContext::setup(&java_sample_path(), true).await?;
        let manager = context
            .manager
            .as_ref()
            .ok_or("Manager is not initialized")?;

        let definition_response = manager
            .find_definition(
                "AStar.java",
                lsp_types::Position {
                    line: 111,
                    character: 8,
                },
            )
            .await?;

        let definitions = match definition_response {
            GotoDefinitionResponse::Scalar(location) => vec![location],
            GotoDefinitionResponse::Array(locations) => locations,
            GotoDefinitionResponse::Link(_links) => Vec::new(),
        };
        let expected = vec![Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/java/AStar.java").unwrap(),
            range: Range {
                start: lsp_types::Position {
                    line: 10,
                    character: 13,
                },
                end: lsp_types::Position {
                    line: 10,
                    character: 18,
                },
            },
        }];

        assert_eq!(definitions, expected);
        Ok(())
    }

    #[tokio::test]
    async fn test_references_js() -> Result<(), Box<dyn std::error::Error>> {
        let context = TestContext::setup(&js_sample_path(), true).await?;
        let manager = context
            .manager
            .as_ref()
            .ok_or("Manager is not initialized")?;

        let file_path = "astar_search.js";

        let references = manager
            .find_references(
                file_path,
                lsp_types::Position {
                    line: 0,
                    character: 9,
                },
            )
            .await?;

        let expected = vec![
            Location {
                uri: Url::parse("file:///mnt/lsproxy_root/sample_project/js/astar_search.js")?,
                range: Range {
                    start: lsp_types::Position {
                        line: 0,
                        character: 9,
                    },
                    end: lsp_types::Position {
                        line: 0,
                        character: 18,
                    },
                },
            },
            Location {
                uri: Url::parse("file:///mnt/lsproxy_root/sample_project/js/astar_search.js")?,
                range: Range {
                    start: lsp_types::Position {
                        line: 10,
                        character: 21,
                    },
                    end: lsp_types::Position {
                        line: 10,
                        character: 30,
                    },
                },
            },
            Location {
                uri: Url::parse("file:///mnt/lsproxy_root/sample_project/js/astar_search.js")?,
                range: Range {
                    start: lsp_types::Position {
                        line: 40,
                        character: 25,
                    },
                    end: lsp_types::Position {
                        line: 40,
                        character: 34,
                    },
                },
            },
        ];
        assert_eq!(references, expected);
        Ok(())
    }

    #[tokio::test]
    async fn test_definition_js() -> Result<(), Box<dyn std::error::Error>> {
        let context = TestContext::setup(&js_sample_path(), true).await?;
        let manager = context
            .manager
            .as_ref()
            .ok_or("Manager is not initialized")?;
        let def_response = manager
            .find_definition(
                "astar_search.js",
                lsp_types::Position {
                    line: 1,
                    character: 18,
                },
            )
            .await?;

        let definitions = match def_response {
            GotoDefinitionResponse::Scalar(location) => vec![location],
            GotoDefinitionResponse::Array(locations) => locations,
            GotoDefinitionResponse::Link(_links) => Vec::new(),
        };

        assert_eq!(
            definitions,
            vec![Location {
                uri: Url::parse("file:///usr/lib/node_modules/typescript/lib/lib.es5.d.ts")?,
                range: Range {
                    start: lsp_types::Position {
                        line: 681,
                        character: 4
                    },
                    end: lsp_types::Position {
                        line: 681,
                        character: 7
                    }
                }
            }]
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_workspace_files_rust() -> Result<(), Box<dyn std::error::Error>> {
        let context = TestContext::setup(&rust_sample_path(), true).await?;

        let manager = context
            .manager
            .as_ref()
            .ok_or("Manager is not initialized")?;
        let files = manager.list_files().await?;

        assert_eq!(
            files,
            vec![
                "src/astar.rs",
                "src/main.rs",
                "src/map.rs",
                "src/node.rs",
                "src/point.rs"
            ]
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_references_rust() -> Result<(), Box<dyn std::error::Error>> {
        let context = TestContext::setup(&rust_sample_path(), true).await?;
        let manager = context
            .manager
            .as_ref()
            .ok_or("Manager is not initialized")?;

        let file_path = "src/node.rs";

        sleep(Duration::from_secs(5)).await;

        let mut references = manager
            .find_references(
                file_path,
                lsp_types::Position {
                    line: 3,
                    character: 11,
                },
            )
            .await?;

        references.sort_by(|a, b| {
            a.uri.to_string().cmp(&b.uri.to_string()).then_with(|| {
                a.range
                    .start
                    .line
                    .cmp(&b.range.start.line)
                    .then_with(|| a.range.start.character.cmp(&b.range.start.character))
            })
        });
        let mut expected = vec![
            Location {
                uri: Url::parse("file:///mnt/lsproxy_root/sample_project/rust/src/node.rs")?,
                range: Range {
                    start: lsp_types::Position {
                        line: 3,
                        character: 11,
                    },
                    end: lsp_types::Position {
                        line: 3,
                        character: 15,
                    },
                },
            },
            Location {
                uri: Url::parse("file:///mnt/lsproxy_root/sample_project/rust/src/node.rs")?,
                range: Range {
                    start: lsp_types::Position {
                        line: 10,
                        character: 20,
                    },
                    end: lsp_types::Position {
                        line: 10,
                        character: 24,
                    },
                },
            },
            Location {
                uri: Url::parse("file:///mnt/lsproxy_root/sample_project/rust/src/node.rs")?,
                range: Range {
                    start: lsp_types::Position {
                        line: 11,
                        character: 34,
                    },
                    end: lsp_types::Position {
                        line: 11,
                        character: 38,
                    },
                },
            },
            Location {
                uri: Url::parse("file:///mnt/lsproxy_root/sample_project/rust/src/astar.rs")?,
                range: Range {
                    start: lsp_types::Position {
                        line: 1,
                        character: 17,
                    },
                    end: lsp_types::Position {
                        line: 1,
                        character: 21,
                    },
                },
            },
            Location {
                uri: Url::parse("file:///mnt/lsproxy_root/sample_project/rust/src/astar.rs")?,
                range: Range {
                    start: lsp_types::Position {
                        line: 6,
                        character: 14,
                    },
                    end: lsp_types::Position {
                        line: 6,
                        character: 18,
                    },
                },
            },
            Location {
                uri: Url::parse("file:///mnt/lsproxy_root/sample_project/rust/src/astar.rs")?,
                range: Range {
                    start: lsp_types::Position {
                        line: 7,
                        character: 16,
                    },
                    end: lsp_types::Position {
                        line: 7,
                        character: 20,
                    },
                },
            },
            Location {
                uri: Url::parse("file:///mnt/lsproxy_root/sample_project/rust/src/astar.rs")?,
                range: Range {
                    start: lsp_types::Position {
                        line: 59,
                        character: 32,
                    },
                    end: lsp_types::Position {
                        line: 59,
                        character: 36,
                    },
                },
            },
            Location {
                uri: Url::parse("file:///mnt/lsproxy_root/sample_project/rust/src/astar.rs")?,
                range: Range {
                    start: lsp_types::Position {
                        line: 76,
                        character: 35,
                    },
                    end: lsp_types::Position {
                        line: 76,
                        character: 39,
                    },
                },
            },
            Location {
                uri: Url::parse("file:///mnt/lsproxy_root/sample_project/rust/src/astar.rs")?,
                range: Range {
                    start: lsp_types::Position {
                        line: 93,
                        character: 23,
                    },
                    end: lsp_types::Position {
                        line: 93,
                        character: 27,
                    },
                },
            },
        ];
        expected.sort_by(|a, b| {
            a.uri.to_string().cmp(&b.uri.to_string()).then_with(|| {
                a.range
                    .start
                    .line
                    .cmp(&b.range.start.line)
                    .then_with(|| a.range.start.character.cmp(&b.range.start.character))
            })
        });
        assert_eq!(references, expected);
        Ok(())
    }

    #[tokio::test]
    async fn test_definition_rust() -> Result<(), Box<dyn std::error::Error>> {
        let context = TestContext::setup(&rust_sample_path(), true).await?;
        let manager = context
            .manager
            .as_ref()
            .ok_or("Manager is not initialized")?;

        sleep(Duration::from_secs(5)).await;

        let def_response = manager
            .find_definition(
                "src/node.rs",
                lsp_types::Position {
                    line: 3,
                    character: 11,
                },
            )
            .await?;

        let definitions = match def_response {
            GotoDefinitionResponse::Scalar(location) => vec![location],
            GotoDefinitionResponse::Array(locations) => locations,
            GotoDefinitionResponse::Link(_links) => Vec::new(),
        };
        let expected = vec![Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/rust/src/node.rs")?,
            range: Range {
                start: lsp_types::Position {
                    line: 3,
                    character: 11,
                },
                end: lsp_types::Position {
                    line: 3,
                    character: 15,
                },
            },
        }];
        assert_eq!(definitions, expected);

        Ok(())
    }
}
