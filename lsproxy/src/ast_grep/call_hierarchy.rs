use std::path::PathBuf;
use log::debug;
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use lsp_types::{CallHierarchyItem, Location, Position, Range, SymbolKind};

use crate::{api_types::SupportedLanguages, utils::file_utils::{detect_language, uri_to_relative_path_string}};

#[derive(Debug, Serialize, Deserialize)]
struct AstGrepPosition {
    line: usize,
    character: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct AstGrepRange {
    start: AstGrepPosition,
    end: AstGrepPosition,
}

#[derive(Debug, Serialize, Deserialize)]
struct AstGrepMatch {
    #[serde(rename = "type")]
    match_type: String,
    name: Option<String>,
    range: AstGrepRange,
    text: String,
}

pub async fn find_enclosing_function(
    location: &Location,
) -> Result<Option<CallHierarchyItem>, Box<dyn std::error::Error>> {
    let file_path_str = uri_to_relative_path_string(&location.uri);
    let file_path = PathBuf::from(&file_path_str);
    let lang = detect_language(&file_path_str)?;

    // Get language-specific pattern from handler
    let lang_str = lang.to_string().to_lowercase();
    let handler = crate::utils::call_hierarchy::get_call_hierarchy_handler(&lang_str)
        .ok_or_else(|| format!("No call hierarchy handler for language: {}", lang_str))?;
    let pattern = handler.get_enclosing_function_pattern();

    debug!(
        "[find_enclosing_function] Running ast-grep for file: {}, pattern: {}",
        file_path.display(),
        pattern
    );

    // Create a temporary config file with our pattern
    let temp_dir = tempfile::tempdir()?;
    let config_path = temp_dir.path().join("config.yml");
    std::fs::write(
        &config_path,
        format!(
            "rules:\n  - pattern: {}\n    language: {}\nruleDirs: []\n",
            pattern,
            lang.to_string().to_lowercase()
        )
    )?;

    // Build the command
    let mut command = Command::new("ast-grep");
    command
        .arg("scan")
        .arg("--config")
        .arg(&config_path)
        .arg("--json")
        .arg(&file_path);

    debug!(
        "[find_enclosing_function] Command: {:?}",
        command
    );

    let command_result = command.output().await?;

    debug!(
        "[find_enclosing_function] Command exit status: {}",
        command_result.status
    );

    if !command_result.status.success() {
        let error = String::from_utf8_lossy(&command_result.stderr);
        debug!(
            "[find_enclosing_function] Command stderr: {}",
            error
        );
        return Err(format!("ast-grep command failed: {}", error).into());
    }

    let output = String::from_utf8(command_result.stdout)?;
    debug!(
        "[find_enclosing_function] Command stdout: {}",
        output
    );
    let mut matches: Vec<AstGrepMatch> = serde_json::from_str(&output)
        .map_err(|e| format!("Failed to parse JSON: {}\nJSON: {}", e, output))?;

    // Sort by range to handle nested functions (larger ranges first)
    matches.sort_by(|a, b| {
        let a_lines = a.range.end.line - a.range.start.line;
        let b_lines = b.range.end.line - b.range.start.line;
        b_lines.cmp(&a_lines)
    });

    // Find the innermost function that contains our location
    let target_line = location.range.start.line as usize;
    let target_char = location.range.start.character as usize;

    let enclosing = matches.iter().find(|m| {
        let range = &m.range;
        (range.start.line <= target_line && target_line <= range.end.line) &&
        (range.start.line != target_line || range.start.character <= target_char) &&
        (range.end.line != target_line || target_char <= range.end.character)
    });

    if let Some(node) = enclosing {
        // Determine symbol kind using language handler
        let kind = handler.determine_symbol_kind(&node.match_type, &node.text);

        let range = Range {
            start: Position {
                line: node.range.start.line as u32,
                character: node.range.start.character as u32,
            },
            end: Position {
                line: node.range.end.line as u32,
                character: node.range.end.character as u32,
            },
        };

        Ok(Some(CallHierarchyItem {
            name: node.name.clone().unwrap_or_else(|| "anonymous".to_string()),
            kind,
            tags: None,
            detail: Some(format!(
                "{:?} • {}",
                lang,
                file_path.file_name().unwrap().to_string_lossy()
            )),
            uri: location.uri.clone(),
            range,
            selection_range: range,
            data: None,
        }))
    } else {
        Ok(None)
    }
}