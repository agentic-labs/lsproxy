use crate::utils::api_utils::{flatten_nested_symbols, symbol_kind_to_string, uri_to_path_str};
use lsp_types::*;
use serde::{Deserialize, Serialize};
use serde_json;
use std::hash::Hash;
use strum_macros::{Display, EnumString};
use utoipa::ToSchema;

pub const MOUNT_DIR: &str = "/mnt/repo";

/// Enumerates the supported programming languages.
#[derive(
    Debug, EnumString, Display, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema,
)]
#[strum(serialize_all = "lowercase")]
pub enum SupportedLanguages {
    Python,
    #[strum(serialize = "typescript_javascript")]
    TypeScriptJavaScript,
    Rust,
}

/// Represents a position within a file.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FilePosition {
    /// The file path relative to the mount directory.
    #[schema(example = "/src/main.rs")]
    pub path: String,

    /// The line number (0-based).
    #[schema(example = 42)]
    pub line: u32,

    /// The character offset (0-based).
    #[schema(example = 10)]
    pub character: u32,
}

/// Represents a symbol in the codebase.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Symbol {
    /// The name of the symbol.
    #[schema(example = "my_function")]
    pub name: String,

    /// The kind/type of the symbol.
    #[schema(example = "function")]
    pub kind: String,

    /// The starting position of the symbol's identifier.
    pub identifier_start_position: FilePosition,
}

/// Generic API response structure.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApiResponse<T> {
    /// The raw JSON response from the LSP server.
    pub raw_response: serde_json::Value,

    /// The simplified and restructured data from the raw response.
    pub data: T,
}

pub type DefinitionResponse = ApiResponse<Vec<FilePosition>>;
pub type ReferenceResponse = ApiResponse<Vec<FilePosition>>;
pub type SymbolResponse = ApiResponse<Vec<Symbol>>;

impl From<Location> for FilePosition {
    fn from(location: Location) -> Self {
        Self {
            path: uri_to_path_str(location.uri),
            line: location.range.start.line,
            character: location.range.start.character,
        }
    }
}

impl From<LocationLink> for FilePosition {
    fn from(link: LocationLink) -> Self {
        Self {
            path: uri_to_path_str(link.target_uri),
            line: link.target_range.start.line,
            character: link.target_range.start.character,
        }
    }
}

impl From<SymbolInformation> for Symbol {
    fn from(symbol: SymbolInformation) -> Self {
        Self {
            name: symbol.name,
            kind: symbol_kind_to_string(symbol.kind).to_string(),
            identifier_start_position: FilePosition::from(symbol.location),
        }
    }
}

impl From<WorkspaceSymbol> for Symbol {
    fn from(symbol: WorkspaceSymbol) -> Self {
        let (path, line, character) = match symbol.location {
            OneOf::Left(location) => (
                uri_to_path_str(location.uri),
                location.range.start.line,
                location.range.start.character,
            ),
            OneOf::Right(workspace_location) => (uri_to_path_str(workspace_location.uri), 0, 0),
        };

        Self {
            name: symbol.name,
            kind: symbol_kind_to_string(symbol.kind).to_string(),
            identifier_start_position: FilePosition {
                path,
                line,
                character,
            },
        }
    }
}

impl From<GotoDefinitionResponse> for DefinitionResponse {
    fn from(response: GotoDefinitionResponse) -> Self {
        let raw_response = serde_json::to_value(&response).unwrap_or_default();
        let data = match response {
            GotoDefinitionResponse::Scalar(location) => vec![FilePosition::from(location)],
            GotoDefinitionResponse::Array(locations) => {
                locations.into_iter().map(FilePosition::from).collect()
            }
            GotoDefinitionResponse::Link(links) => {
                links.into_iter().map(FilePosition::from).collect()
            }
        };
        Self { raw_response, data }
    }
}

impl From<Vec<Location>> for ReferenceResponse {
    fn from(locations: Vec<Location>) -> Self {
        Self {
            raw_response: serde_json::to_value(&locations).unwrap_or_default(),
            data: locations.into_iter().map(FilePosition::from).collect(),
        }
    }
}

impl From<Vec<WorkspaceSymbolResponse>> for SymbolResponse {
    fn from(responses: Vec<WorkspaceSymbolResponse>) -> Self {
        Self {
            raw_response: serde_json::to_value(&responses).unwrap_or_default(),
            data: responses
                .into_iter()
                .flat_map(|response| match response {
                    WorkspaceSymbolResponse::Flat(symbols) => {
                        symbols.into_iter().map(Symbol::from).collect::<Vec<_>>()
                    }
                    WorkspaceSymbolResponse::Nested(symbols) => {
                        symbols.into_iter().map(Symbol::from).collect()
                    }
                })
                .collect(),
        }
    }
}

impl SymbolResponse {
    /// Creates a new `SymbolResponse` from a `DocumentSymbolResponse`.
    ///
    /// # Arguments
    ///
    /// * `response` - The document symbol response from the LSP server.
    /// * `file_path` - The file path associated with the symbols.
    pub fn new(response: DocumentSymbolResponse, file_path: &str) -> Self {
        Self {
            raw_response: serde_json::to_value(&response).unwrap_or_default(),
            data: match response {
                DocumentSymbolResponse::Flat(symbols) => symbols
                    .into_iter()
                    .map(|symbol| Symbol {
                        name: symbol.name,
                        kind: symbol_kind_to_string(symbol.kind).to_string(),
                        identifier_start_position: FilePosition {
                            path: file_path.to_string(),
                            line: symbol.location.range.start.line,
                            character: symbol.location.range.start.character,
                        },
                    })
                    .collect(),
                DocumentSymbolResponse::Nested(symbols) => {
                    flatten_nested_symbols(symbols, file_path)
                }
            },
        }
    }
}
