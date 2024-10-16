use crate::api_types::{FilePosition, Symbol};
use lsp_types::{DocumentSymbol, SymbolKind};
use std::path::{Path, PathBuf};
use url::Url;

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
        kind: symbol_kind_to_string(symbol.kind).to_string(),
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

/// Converts a `SymbolKind` to its string representation.
///
/// Utilizes `strum` for cleaner enum management.
///
/// # Arguments
///
/// * `kind` - The symbol kind from LSP types.
///
/// # Returns
///
/// A string slice representing the symbol kind.
pub fn symbol_kind_to_string(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::FILE => "file",
        SymbolKind::MODULE => "module",
        SymbolKind::NAMESPACE => "namespace",
        SymbolKind::PACKAGE => "package",
        SymbolKind::CLASS => "class",
        SymbolKind::METHOD => "method",
        SymbolKind::PROPERTY => "property",
        SymbolKind::FIELD => "field",
        SymbolKind::CONSTRUCTOR => "constructor",
        SymbolKind::ENUM => "enum",
        SymbolKind::INTERFACE => "interface",
        SymbolKind::FUNCTION => "function",
        SymbolKind::VARIABLE => "variable",
        SymbolKind::CONSTANT => "constant",
        SymbolKind::STRING => "string",
        SymbolKind::NUMBER => "number",
        SymbolKind::BOOLEAN => "boolean",
        SymbolKind::ARRAY => "array",
        SymbolKind::OBJECT => "object",
        SymbolKind::KEY => "key",
        SymbolKind::NULL => "null",
        SymbolKind::ENUM_MEMBER => "enum_member",
        SymbolKind::STRUCT => "struct",
        SymbolKind::EVENT => "event",
        SymbolKind::OPERATOR => "operator",
        SymbolKind::TYPE_PARAMETER => "type_parameter",
        _ => "unknown",
    }
}
