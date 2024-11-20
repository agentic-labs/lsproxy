use crate::api_types::get_mount_dir;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};
use url::Url;

pub fn search_files(
    path: &std::path::Path,
    include_patterns: Vec<String>,
    exclude_patterns: Vec<String>,
) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    let walk = build_walk(path, exclude_patterns);
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
    let walk = build_walk(root_path, exclude_patterns);
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

fn build_walk(path: &Path, exclude_patterns: Vec<String>) -> ignore::Walk {
    let walk = WalkBuilder::new(path)
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

pub fn uri_to_relative_path_string(
    uri: &Url,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let path = uri
        .to_file_path()
        .map_err(|()| "Failed to convert URI to file path")?;
    absolute_path_to_relative_path_string(&path)
}

pub fn absolute_path_to_relative_path_string(
    path: &PathBuf,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mount_dir = get_mount_dir();
    path.strip_prefix(mount_dir)
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| format!("Failed to strip prefix from {:?}: {:?}", path, e).into())
}
