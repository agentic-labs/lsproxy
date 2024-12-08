use serde::{Deserialize, Serialize};
use tokio::fs;

use std::collections::HashSet;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use crate::utils::file_utils::{search_directories, search_files};
use crate::utils::workspace_documents::DidOpenConfiguration;
use crate::{
    lsp::{JsonRpcHandler, LspClient, PendingRequests, ProcessHandler},
    utils::workspace_documents::{
        WorkspaceDocumentsHandler, CPP_ROOT_FILES, C_AND_CPP_FILE_PATTERNS,
        DEFAULT_EXCLUDE_PATTERNS,
    },
};
use async_trait::async_trait;
use fs::write;
use log::debug;
use lsp_types::InitializeParams;
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
            vec![String::from("**/compile_commands.json")],
            vec![String::from("**/.git")],
            false,
        )?;

        if compile_db_files.is_empty() {
            debug!("Couldn't find compile comands json, falling back to generation");
            // this is a workaround to avoid building the entire project
            let commands = generate_compile_commands(root_path.to_string())?;

            let json = serde_json::to_string_pretty(&commands)?;

            write(Path::new(root_path).join("compile_commands.json"), json).await?;

            debug!(
                "Generated compile_commands.json with {} entries",
                commands.len()
            );
        }
        Ok(())
    }

    async fn get_initialize_params(&mut self, root_path: String) -> InitializeParams {
        let capabilities = self.get_capabilities();
        InitializeParams {
            capabilities,
            root_uri: Some(Url::from_file_path(root_path).unwrap()),
            initialization_options: Some(serde_json::json!({
                "clangdFileStatus": true, // TODO: actually wait for the status when hitting a file
            })),
            ..Default::default()
        }
    }
}

impl ClangdClient {
    pub async fn new(
        root_path: &str,
        watch_events_rx: Receiver<DebouncedEvent>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let debug_file = std::fs::File::create("/tmp/clangd.log")?;

        let process = Command::new("clangd")
            .arg("--log=info")
            .current_dir(root_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(debug_file)
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
            DidOpenConfiguration::Lazy,
        );
        let pending_requests = PendingRequests::new();

        Ok(Self {
            process: process_handler,
            json_rpc: json_rpc_handler,
            workspace_documents,
            pending_requests,
        })
    }
}

#[derive(Serialize, Deserialize)]
struct CompileCommand {
    directory: String,
    command: String,
    file: String,
}

fn find_include_dirs(project_root: &Path, cmakelists_files: &[PathBuf]) -> Vec<String> {
    let mut include_dirs = HashSet::new();

    // Use search_directories to find all directories (including "include")
    let include_patterns = vec!["**/*include*".to_string()]; // Matches any directory with "include" as a substring
    let exclude_patterns: Vec<String> = DEFAULT_EXCLUDE_PATTERNS
        .iter()
        .map(|&s| s.to_string())
        .collect();

    if let Ok(dirs) = search_directories(project_root, include_patterns, exclude_patterns) {
        for dir in dirs {
            // Only add the directory itself, not its subdirectories
            if dir.is_dir() {
                include_dirs.insert(dir.to_string_lossy().to_string());
            }
        }
    }

    // Add directories containing CMakeLists.txt files
    for cmake_file in cmakelists_files {
        if let Some(parent_dir) = cmake_file.parent() {
            include_dirs.insert(parent_dir.to_string_lossy().into_owned());
        }
    }

    include_dirs.into_iter().collect()
}

fn find_source_files(project_root: &Path) -> Vec<String> {
    let include_patterns = vec![
        "**/*.cpp".to_string(),
        "**/*.cc".to_string(),
        "**/*.cxx".to_string(),
        "**/*.c".to_string(),
    ];
    let exclude_patterns: Vec<String> = DEFAULT_EXCLUDE_PATTERNS
        .iter()
        .map(|&s| s.to_string())
        .collect();

    match search_files(project_root, include_patterns, exclude_patterns, true) {
        Ok(files) => files
            .into_iter()
            .map(|file| file.to_string_lossy().into_owned())
            .collect(),
        Err(err) => {
            debug!("Error finding source files: {}", err);
            vec![]
        }
    }
}

fn generate_compile_commands(
    project_root: String,
) -> Result<Vec<CompileCommand>, Box<dyn std::error::Error + Send + Sync>> {
    let project_path = Path::new(&project_root);

    // Find CMakeLists.txt files
    debug!("Finding CMakeLists.txt files...");
    let cmakelists_files = search_files(
        project_path,
        vec!["**/CMakeLists.txt".to_string()],
        DEFAULT_EXCLUDE_PATTERNS
            .iter()
            .map(|&s| s.to_string())
            .collect(),
        true,
    )?;

    // Parse CMakeLists.txt for compiler flags and C++ standard
    debug!("Parsing CMakeLists.txt files...");
    let flags = parse_cmakelists(&cmakelists_files);

    // Find inferred include directories
    debug!("Finding inferred include directories...");
    let include_dirs = find_include_dirs(project_path, &cmakelists_files);

    // Find source files
    debug!("Finding source files...");
    let source_files = find_source_files(project_path);

    debug!("Found {} source files", source_files.len());
    debug!("Using include paths: {:?}", include_dirs);
    debug!("Using compiler flags: {:?}", flags);

    // Generate compile commands
    let compiler = "/usr/bin/c++";
    let include_flags: Vec<String> = include_dirs
        .iter()
        .map(|inc| format!("-I{}", inc))
        .collect();

    let compile_commands = source_files
        .iter()
        .map(|file| CompileCommand {
            directory: project_root.clone(),
            command: format!(
                "{} {} {} -c {}",
                compiler,
                include_flags.join(" "),
                flags.join(" "),
                file
            ),
            file: file.clone(),
        })
        .collect();

    Ok(compile_commands)
}

fn parse_cmakelists(cmake_files: &[PathBuf]) -> Vec<String> {
    let mut flags = Vec::new();
    for cmake_path in cmake_files {
        if let Ok(content) = std::fs::read_to_string(cmake_path) {
            // Extract C++ standard (this part is fine)
            if let Some(capture) = regex::Regex::new(r"set\s*\(\s*CMAKE_CXX_STANDARD\s+(\d+)\s*\)")
                .unwrap()
                .captures(&content)
            {
                flags.push(format!("-std=c++{}", &capture[1]));
            }

            // Extract compile options but skip generator expressions and variables
            for caps in regex::Regex::new(r"add_compile_options\s*\((.*?)\)")
                .unwrap()
                .captures_iter(&content)
            {
                // Only take literal flags, skip anything with ${...} or $<...>
                flags.extend(
                    caps[1]
                        .split_whitespace()
                        .filter(|arg| !arg.contains("${") && !arg.contains("$<"))
                        .map(String::from),
                );
            }
        }
    }
    flags
}
