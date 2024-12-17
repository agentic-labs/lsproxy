use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use lsp_types::{CallHierarchyItem, Location, Position, Range, SymbolKind};

use crate::utils::file_utils::{detect_language, uri_to_relative_path_string};

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

    // Run ast-grep to find all function-like declarations
    let command_result = Command::new("ast-grep")
        .arg("scan")
        .arg("--pattern")
        .arg("(function_declaration | method_declaration | class_declaration | impl_item) @cap")
        .arg("--json")
        .arg(&file_path)
        .output()
        .await?;

    if !command_result.status.success() {
        let error = String::from_utf8_lossy(&command_result.stderr);
        return Err(format!("ast-grep command failed: {}", error).into());
    }

    let output = String::from_utf8(command_result.stdout)?;
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
        // Determine symbol kind based on match type and content
        let kind = match node.match_type.as_str() {
            "class_declaration" => SymbolKind::CLASS,
            "impl_item" => SymbolKind::CLASS,
            _ if node.text.contains("self") || node.text.contains("this") => SymbolKind::METHOD,
            _ => SymbolKind::FUNCTION,
        };

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