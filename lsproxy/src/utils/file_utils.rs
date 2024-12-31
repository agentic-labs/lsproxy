use crate::{
    api_types::{get_mount_dir, SupportedLanguages},
    lsp::manager::LspManagerError,
};
use ignore::WalkBuilder;
use log::warn;
use std::path::{Path, PathBuf};
use url::Url;

use super::workspace_documents::{
    CPP_EXTENSIONS, C_AND_CPP_EXTENSIONS, C_EXTENSIONS, GOLANG_EXTENSIONS, JAVASCRIPTREACT_EXTENSIONS, JAVASCRIPT_EXTENSIONS, JAVA_EXTENSIONS, PHP_EXTENSIONS, PYTHON_EXTENSIONS, RUST_EXTENSIONS, TYPESCRIPTREACT_EXTENSIONS, TYPESCRIPT_AND_JAVASCRIPT_EXTENSIONS, TYPESCRIPT_EXTENSIONS
};

pub fn search_files(
    path: &std::path::Path,
    include_patterns: Vec<String>,
    exclude_patterns: Vec<String>,
    respect_gitignore: bool,
) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    let walk = build_walk(path, exclude_patterns, respect_gitignore);
    // println!("Searching for {:?}",include_patterns);
    for result in walk {
        match result {
            Ok(entry) => {
                let path = entry.path();
                if !include_patterns.iter().any(|pattern| {
                    glob::Pattern::new(pattern)
                        .map(|p| p.matches_path(&path))
                        .unwrap_or(false)
                }) {
                    continue;
                }
                if path.is_file() {
                    files.push(path.to_path_buf());
                }
            }
            Err(err) => eprintln!("Error: {}", err),
        }
    }

    Ok(files)
}

pub fn search_directories(
    root_path: &std::path::Path,
    include_patterns: Vec<String>,
    exclude_patterns: Vec<String>,
) -> std::io::Result<Vec<PathBuf>> {
    let mut dirs = Vec::new();
    let walk = build_walk(root_path, exclude_patterns, true);
    for result in walk {
        match result {
            Ok(entry) => {
                let path = entry.path().to_path_buf();
                if !include_patterns.iter().any(|pattern| {
                    glob::Pattern::new(pattern)
                        .map(|p| p.matches_path(&path))
                        .unwrap_or(false)
                }) {
                    continue;
                }
                if path.is_dir() {
                    dirs.push(path);
                } else {
                    dirs.push(path.parent().unwrap().to_path_buf());
                }
            }
            Err(err) => eprintln!("Error: {}", err),
        }
    }
    Ok(dirs
        .into_iter()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect())
}

fn build_walk(path: &Path, exclude_patterns: Vec<String>, respect_gitignore: bool) -> ignore::Walk {
    let walk = WalkBuilder::new(path)
        .git_ignore(respect_gitignore)
        .filter_entry(move |entry| {
            let path = entry.path();
            let is_excluded = exclude_patterns.iter().any(|pattern| {
                let matches = glob::Pattern::new(pattern)
                    .map(|p| p.matches_path(path))
                    .unwrap_or(false);
                matches
            });
            !is_excluded
        })
        .build();
    walk
}

pub fn uri_to_relative_path_string(uri: &Url) -> String {
    let path = uri.to_file_path().unwrap_or_else(|e| {
        warn!("Failed to convert URI to file path: {:?}", e);
        PathBuf::from(uri.path())
    });

    absolute_path_to_relative_path_string(&path)
}

pub fn absolute_path_to_relative_path_string(path: &PathBuf) -> String {
    let mount_dir = get_mount_dir();
    path.strip_prefix(mount_dir)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|e| {
            warn!("Failed to strip prefix from {:?}: {:?}", path, e);
            path.to_string_lossy().into_owned()
        })
}

pub fn detect_language(file_path: &str) -> Result<SupportedLanguages, LspManagerError> {
    let path = PathBuf::from(file_path);
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| LspManagerError::UnsupportedFileType(file_path.to_string()))?;

    match extension {
        ext if PYTHON_EXTENSIONS.contains(&ext) => Ok(SupportedLanguages::Python),
        ext if TYPESCRIPT_AND_JAVASCRIPT_EXTENSIONS.contains(&ext) => {
            Ok(SupportedLanguages::TypeScriptJavaScript)
        }
        ext if RUST_EXTENSIONS.contains(&ext) => Ok(SupportedLanguages::Rust),
        ext if C_AND_CPP_EXTENSIONS.contains(&ext) => Ok(SupportedLanguages::CPP),
        ext if JAVA_EXTENSIONS.contains(&ext) => Ok(SupportedLanguages::Java),
        ext if GOLANG_EXTENSIONS.contains(&ext) => Ok(SupportedLanguages::Golang),
        ext if PHP_EXTENSIONS.contains(&ext) => Ok(SupportedLanguages::PHP),
        _ => Err(LspManagerError::UnsupportedFileType(file_path.to_string())),
    }
}

pub fn detect_language_string(file_path: &str) -> Result<String, LspManagerError> {
    let path = PathBuf::from(file_path);
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| LspManagerError::UnsupportedFileType(file_path.to_string()))?;

    match extension {
        ext if PYTHON_EXTENSIONS.contains(&ext) => Ok("python".to_string()),
        ext if TYPESCRIPT_EXTENSIONS.contains(&ext) => Ok("typescript".to_string()),
        ext if TYPESCRIPTREACT_EXTENSIONS.contains(&ext) => Ok("typescriptreact".to_string()),
        ext if JAVASCRIPT_EXTENSIONS.contains(&ext) => Ok("javascript".to_string()),
        ext if JAVASCRIPTREACT_EXTENSIONS.contains(&ext) => Ok("javascriptreact".to_string()),
        ext if RUST_EXTENSIONS.contains(&ext) => Ok("rust".to_string()),
        ext if C_EXTENSIONS.contains(&ext) => Ok("c".to_string()),
        ext if CPP_EXTENSIONS.contains(&ext) => Ok("cpp".to_string()),
        ext if JAVA_EXTENSIONS.contains(&ext) => Ok("java".to_string()),
        ext if GOLANG_EXTENSIONS.contains(&ext) => Ok("golang".to_string()),
        ext if PHP_EXTENSIONS.contains(&ext) => Ok("php".to_string()),
        _ => Err(LspManagerError::UnsupportedFileType(file_path.to_string())),
    }
}
