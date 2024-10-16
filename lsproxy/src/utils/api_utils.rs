use crate::api_types::{FilePosition, Symbol};
use lsp_types::DocumentSymbol;
use std::path::{Path, PathBuf};

const MOUNT_DIR: &str = "/mnt/repo";

/// Converts a `Url` to a file system path string relative to the mount directory.
///
/// # Arguments
///
/// * `uri` - The URL to convert.
///
/// # Returns
///
/// A `String` representing the relative file path.
pub fn uri_to_path_str(uri: Url) -> String {
    uri.to_file_path()
        .map_or_else(|_| PathBuf::from(uri.path()), |path| path)
        .strip_prefix(Path::new(MOUNT_DIR))
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| uri.path().to_owned())
}

/// Flattens nested `DocumentSymbol` structures into a flat vector of `Symbol`.
///
/// # Arguments
///
/// * `symbols` - The vector of `DocumentSymbol` to flatten.
/// * `file_path` - The file path associated with the symbols.
///
/// # Returns
///
/// A flat `Vec<Symbol>` extracted from the nested symbols.
pub fn flatten_nested_symbols(symbols: Vec<DocumentSymbol>, file_path: &str) -> Vec<Symbol> {
    symbols
        .into_iter()
        .flat_map(|symbol| recursive_flatten(symbol, file_path))
        .collect()
}

/// Recursively flattens a single `DocumentSymbol` into a vector of `Symbol`.
///
/// # Arguments
///
/// * `symbol` - The `DocumentSymbol` to flatten.
/// * `file_path` - The file path associated with the symbol.
///
/// # Returns
///
/// A `Vec<Symbol>` representing the flattened symbols.
fn recursive_flatten(symbol: DocumentSymbol, file_path: &str) -> Vec<Symbol> {
    let mut result = vec![Symbol {
        name: symbol.name,
        kind: symbol_kind_to_string(&symbol.kind).to_string(),
        identifier_start_position: FilePosition {
            path: file_path.to_string(),
            line: symbol.selection_range.start.line,
            character: symbol.selection_range.start.character,
        },
    }];

    if let Some(children) = symbol.children {
        for child in children {
            result.extend(recursive_flatten(child, file_path));
        }
    }
    result
}
