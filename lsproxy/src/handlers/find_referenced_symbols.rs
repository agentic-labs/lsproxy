use crate::api_types::{
    ErrorResponse, FilePosition, GetReferencedSymbolsRequest, Identifier, Position,
    ReferenceWithSymbolDefinitions, ReferencedSymbolsResponse,
};
use crate::utils::file_utils::uri_to_relative_path_string;
use crate::AppState;
use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use log::{error, info};
use lsp_types::{GotoDefinitionResponse, Position as LspPosition};

/// Find all symbols that are referenced from a given position
///
/// Returns references categorized into:
/// - Workspace symbols (with their definitions)
/// - External symbols (built-in functions like 'len', 'print' or from external libraries)
/// - Symbols that couldn't be found
///
/// e.g. for a function in `main.py`:
/// ```python
/// @log_execution_time     # Reference to decorator
/// def process_user():
///     user = User()       # Reference to User class
///     print("Done")       # Reference to built-in function
/// ```
/// This would return:
/// - Workspace symbols: log_execution_time (with definition from decorators.py)
/// - Workspace symbols: User (with definition from models.py)
/// - External symbols: print (Python built-in)
#[utoipa::path(
    post,
    path = "/symbol/find-referenced-symbols",
    tag = "symbol",
    request_body = GetReferencedSymbolsRequest,
    responses(
        (status = 200, description = "Referenced symbols retrieved successfully", body = ReferencedSymbolsResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn find_referenced_symbols(
    data: Data<AppState>,
    info: Json<GetReferencedSymbolsRequest>,
) -> HttpResponse {
    info!(
        "Received referenced symbols request for file: {}, line: {}, character: {}",
        info.identifier_position.path,
        info.identifier_position.position.line,
        info.identifier_position.position.character
    );

    let manager = data.manager.lock().unwrap();

    let referenecd_ast_symbols = match manager
        .find_referenced_symbols(
            &info.identifier_position.path,
            LspPosition {
                line: info.identifier_position.position.line,
                character: info.identifier_position.position.character,
            },
        )
        .await
    {
        Ok(ast_symbols) => ast_symbols,
        Err(e) => {
            error!("Failed to get referenced symbols: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to get referenced symbols: {}", e),
            });
        }
    };

    let unwrapped_definition_responses: Vec<(Identifier, Vec<FilePosition>)> =
        referenecd_ast_symbols
            .into_iter()
            .map(|(ast_grep_result, definition_response)| {
                let definitions = match definition_response {
                    GotoDefinitionResponse::Scalar(location) => vec![FilePosition {
                        path: uri_to_relative_path_string(&location.uri),
                        position: Position {
                            line: location.range.start.line,
                            character: location.range.start.character,
                        },
                    }],
                    GotoDefinitionResponse::Array(locations) => locations
                        .into_iter()
                        .map(|location| FilePosition {
                            path: uri_to_relative_path_string(&location.uri),
                            position: Position {
                                line: location.range.start.line,
                                character: location.range.start.character,
                            },
                        })
                        .collect(),
                    GotoDefinitionResponse::Link(links) => links
                        .into_iter()
                        .map(|link| FilePosition {
                            path: uri_to_relative_path_string(&link.target_uri),
                            position: Position {
                                line: link.target_range.start.line,
                                character: link.target_range.start.character,
                            },
                        })
                        .collect(),
                };
                (Identifier::from(ast_grep_result), definitions)
            })
            .collect();

    // First get the workspace files
    let files = match manager.list_files().await {
        Ok(files) => files,
        Err(e) => {
            error!("Failed to list workspace files: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to list workspace files: {}", e),
            });
        }
    };

    // Then categorize the definitions
    let mut workspace_symbols = Vec::new();
    let mut external_symbols = Vec::new();
    let mut not_found = Vec::new();

    for (identifier, definitions) in unwrapped_definition_responses {
        if definitions.is_empty() {
            not_found.push(identifier);
        } else {
            // Check if any definition is in workspace files
            let has_internal_definition = definitions.iter().any(|def| files.contains(&def.path));
            if has_internal_definition {
                let mut symbols_with_definitions = Vec::new();
                for def in definitions.iter().filter(|def| files.contains(&def.path)) {
                    if let Ok(symbol) = manager
                        .get_symbol_from_position(
                            &def.path,
                            &lsp_types::Position {
                                line: def.position.line,
                                character: def.position.character,
                            },
                        )
                        .await
                    {
                        symbols_with_definitions.push(symbol);
                    }
                }
                // Only add to workspace_symbols if we found at least one symbol
                if !symbols_with_definitions.is_empty() {
                    workspace_symbols.push(ReferenceWithSymbolDefinitions {
                        reference: identifier.clone(),
                        definitions: symbols_with_definitions,
                    });
                } else {
                    // If no symbols were found, add to not_found
                    not_found.push(identifier.clone());
                }
            } else {
                external_symbols.push(identifier.clone());
            }
        }
    }

    // Sort workspace_symbols by reference location
    workspace_symbols.sort_by(|a, b| {
        let path_cmp = a.reference.range.path.cmp(&b.reference.range.path);
        if path_cmp.is_eq() {
            a.reference
                .range
                .start
                .line
                .cmp(&b.reference.range.start.line)
        } else {
            path_cmp
        }
    });

    // Sort external_symbols by location
    external_symbols.sort_by(|a, b| {
        let path_cmp = a.range.path.cmp(&b.range.path);
        if path_cmp.is_eq() {
            a.range.start.line.cmp(&b.range.start.line)
        } else {
            path_cmp
        }
    });

    // Sort not_found by location
    not_found.sort_by(|a, b| {
        let path_cmp = a.range.path.cmp(&b.range.path);
        if path_cmp.is_eq() {
            a.range.start.line.cmp(&b.range.start.line)
        } else {
            path_cmp
        }
    });

    // Return the sorted response
    HttpResponse::Ok().json(ReferencedSymbolsResponse {
        workspace_symbols,
        external_symbols,
        not_found,
    })
}

#[cfg(test)]
mod test {
    use super::*;

    use actix_web::http::StatusCode;
    use tokio::time::{sleep, Duration};

    use crate::api_types::{FilePosition, FileRange, Position, Symbol};
    use crate::initialize_app_state;
    use crate::test_utils::{python_sample_path, TestContext};

    #[tokio::test]
    async fn test_python_nested_function_referenced_symbols(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup(&python_sample_path(), false).await?;
        let state = initialize_app_state().await?;

        let mock_request = Json(GetReferencedSymbolsRequest {
            identifier_position: FilePosition {
                path: String::from("search.py"),
                position: Position {
                    line: 16,
                    character: 4,
                },
            },
        });

        sleep(Duration::from_secs(5)).await;

        let response = find_referenced_symbols(state, mock_request).await;
        assert_eq!(response.status(), StatusCode::OK, "Response: {:?}", response);
        let content_type = response
            .headers()
            .get("content-type")
            .ok_or("Missing content-type header")?
            .to_str()?;
        assert_eq!(content_type, "application/json");

        // Check the body
        let body = response.into_body();
        let bytes = actix_web::body::to_bytes(body).await?;
        let referenced_symbols_response: ReferencedSymbolsResponse =
            serde_json::from_slice(&bytes)?;

        let expected_response = ReferencedSymbolsResponse {
            workspace_symbols: vec![
                ReferenceWithSymbolDefinitions {
                    reference: Identifier {
                        name: String::from("log_execution_time"),
                        range: FileRange {
                            path: String::from("search.py"),
                            start: Position {
                                line: 15,
                                character: 1,
                            },
                            end: Position {
                                line: 15,
                                character: 19,
                            },
                        },
                        kind: Some(String::from("decorator")),
                    },
                    definitions: vec![Symbol {
                        name: String::from("log_execution_time"),
                        kind: String::from("function"),
                        identifier_position: FilePosition {
                            path: String::from("decorators.py"),
                            position: Position {
                                line: 3,
                                character: 4,
                            },
                        },
                        range: FileRange {
                            path: String::from("decorators.py"),
                            start: Position {
                                line: 3,
                                character: 0,
                            },
                            end: Position {
                                line: 11,
                                character: 18,
                            },
                        },
                    }],
                },
                ReferenceWithSymbolDefinitions {
                    reference: Identifier {
                        name: String::from("initialize_search"),
                        range: FileRange {
                            path: String::from("search.py"),
                            start: Position {
                                line: 29,
                                character: 54,
                            },
                            end: Position {
                                line: 29,
                                character: 71,
                            },
                        },
                        kind: Some(String::from("function-call")),
                    },
                    definitions: vec![Symbol {
                        name: String::from("initialize_search"),
                        kind: String::from("function"),
                        identifier_position: FilePosition {
                            path: String::from("search.py"),
                            position: Position {
                                line: 5,
                                character: 4,
                            },
                        },
                        range: FileRange {
                            path: String::from("search.py"),
                            start: Position {
                                line: 4,
                                character: 0,
                            },
                            end: Position {
                                line: 13,
                                character: 58,
                            },
                        },
                    }],
                },
                ReferenceWithSymbolDefinitions {
                    reference: Identifier {
                        name: String::from("reconstruct_path"),
                        range: FileRange {
                            path: String::from("search.py"),
                            start: Position {
                                line: 36,
                                character: 19,
                            },
                            end: Position {
                                line: 36,
                                character: 35,
                            },
                        },
                        kind: Some(String::from("function-call")),
                    },
                    definitions: vec![Symbol {
                        name: String::from("reconstruct_path"),
                        kind: String::from("function"),
                        identifier_position: FilePosition {
                            path: String::from("search.py"),
                            position: Position {
                                line: 17,
                                character: 8,
                            },
                        },
                        range: FileRange {
                            path: String::from("search.py"),
                            start: Position {
                                line: 17,
                                character: 0,
                            },
                            end: Position {
                                line: 27,
                                character: 25,
                            },
                        },
                    }],
                },
                ReferenceWithSymbolDefinitions {
                    reference: Identifier {
                        name: String::from("get_vertex_neighbours"),
                        range: FileRange {
                            path: String::from("search.py"),
                            start: Position {
                                line: 41,
                                character: 31,
                            },
                            end: Position {
                                line: 41,
                                character: 52,
                            },
                        },
                        kind: Some(String::from("function-call")),
                    },
                    definitions: vec![Symbol {
                        name: String::from("get_vertex_neighbours"),
                        kind: String::from("function"),
                        identifier_position: FilePosition {
                            path: String::from("graph.py"),
                            position: Position {
                                line: 76,
                                character: 8,
                            },
                        },
                        range: FileRange {
                            path: String::from("graph.py"),
                            start: Position {
                                line: 75,
                                character: 0,
                            },
                            end: Position {
                                line: 88,
                                character: 16,
                            },
                        },
                    }],
                },
                ReferenceWithSymbolDefinitions {
                    reference: Identifier {
                        name: String::from("move_cost"),
                        range: FileRange {
                            path: String::from("search.py"),
                            start: Position {
                                line: 45,
                                character: 45,
                            },
                            end: Position {
                                line: 45,
                                character: 54,
                            },
                        },
                        kind: Some(String::from("function-call")),
                    },
                    definitions: vec![Symbol {
                        name: String::from("move_cost"),
                        kind: String::from("function"),
                        identifier_position: FilePosition {
                            path: String::from("graph.py"),
                            position: Position {
                                line: 43,
                                character: 8,
                            },
                        },
                        range: FileRange {
                            path: String::from("graph.py"),
                            start: Position {
                                line: 43,
                                character: 0,
                            },
                            end: Position {
                                line: 65,
                                character: 34,
                            },
                        },
                    }],
                },
                ReferenceWithSymbolDefinitions {
                    reference: Identifier {
                        name: String::from("heuristic"),
                        range: FileRange {
                            path: String::from("search.py"),
                            start: Position {
                                line: 54,
                                character: 48,
                            },
                            end: Position {
                                line: 54,
                                character: 57,
                            },
                        },
                        kind: Some(String::from("function-call")),
                    },
                    definitions: vec![Symbol {
                        name: String::from("heuristic"),
                        kind: String::from("function"),
                        identifier_position: FilePosition {
                            path: String::from("graph.py"),
                            position: Position {
                                line: 68,
                                character: 8,
                            },
                        },
                        range: FileRange {
                            path: String::from("graph.py"),
                            start: Position {
                                line: 67,
                                character: 0,
                            },
                            end: Position {
                                line: 73,
                                character: 57,
                            },
                        },
                    }],
                },
            ],
            external_symbols: vec![
                Identifier {
                    name: String::from("append"),
                    range: FileRange {
                        path: String::from("search.py"),
                        start: Position {
                            line: 24,
                            character: 17,
                        },
                        end: Position {
                            line: 24,
                            character: 23,
                        },
                    },
                    kind: Some(String::from("function-call")),
                },
                Identifier {
                    name: String::from("append"),
                    range: FileRange {
                        path: String::from("search.py"),
                        start: Position {
                            line: 26,
                            character: 13,
                        },
                        end: Position {
                            line: 26,
                            character: 19,
                        },
                    },
                    kind: Some(String::from("function-call")),
                },
                Identifier {
                    name: String::from("min"),
                    range: FileRange {
                        path: String::from("search.py"),
                        start: Position {
                            line: 34,
                            character: 18,
                        },
                        end: Position {
                            line: 34,
                            character: 21,
                        },
                    },
                    kind: Some(String::from("function-call")),
                },
                Identifier {
                    name: String::from("remove"),
                    range: FileRange {
                        path: String::from("search.py"),
                        start: Position {
                            line: 38,
                            character: 22,
                        },
                        end: Position {
                            line: 38,
                            character: 28,
                        },
                    },
                    kind: Some(String::from("function-call")),
                },
                Identifier {
                    name: String::from("add"),
                    range: FileRange {
                        path: String::from("search.py"),
                        start: Position {
                            line: 39,
                            character: 24,
                        },
                        end: Position {
                            line: 39,
                            character: 27,
                        },
                    },
                    kind: Some(String::from("function-call")),
                },
                Identifier {
                    name: String::from("add"),
                    range: FileRange {
                        path: String::from("search.py"),
                        start: Position {
                            line: 48,
                            character: 30,
                        },
                        end: Position {
                            line: 48,
                            character: 33,
                        },
                    },
                    kind: Some(String::from("function-call")),
                },
                Identifier {
                    name: String::from("get"),
                    range: FileRange {
                        path: String::from("search.py"),
                        start: Position {
                            line: 49,
                            character: 34,
                        },
                        end: Position {
                            line: 49,
                            character: 37,
                        },
                    },
                    kind: Some(String::from("function-call")),
                },
                Identifier {
                    name: String::from("float"),
                    range: FileRange {
                        path: String::from("search.py"),
                        start: Position {
                            line: 49,
                            character: 49,
                        },
                        end: Position {
                            line: 49,
                            character: 54,
                        },
                    },
                    kind: Some(String::from("function-call")),
                },
                Identifier {
                    name: String::from("RuntimeError"),
                    range: FileRange {
                        path: String::from("search.py"),
                        start: Position {
                            line: 56,
                            character: 10,
                        },
                        end: Position {
                            line: 56,
                            character: 22,
                        },
                    },
                    kind: Some(String::from("function-call")),
                },
            ],
            not_found: vec![],
        };

        assert_eq!(referenced_symbols_response, expected_response);
        Ok(())
    }

    #[tokio::test]
    async fn test_python_class_referenced_symbols() -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup(&python_sample_path(), false).await?;
        let state = initialize_app_state().await?;

        let mock_request = Json(GetReferencedSymbolsRequest {
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 12,
                    character: 6,
                },
            },
        });

        sleep(Duration::from_secs(5)).await;

        let response = find_referenced_symbols(state, mock_request).await;
        assert_eq!(response.status(), StatusCode::OK, "Response: {:?}", response);
        let content_type = response
            .headers()
            .get("content-type")
            .ok_or("Missing content-type header")?
            .to_str()?;
        assert_eq!(content_type, "application/json");

        // Check the body
        let body = response.into_body();
        let bytes = actix_web::body::to_bytes(body).await?;
        let referenced_symbols_response: ReferencedSymbolsResponse =
            serde_json::from_slice(&bytes)?;

        let expected_response = ReferencedSymbolsResponse {
            workspace_symbols: vec![
                ReferenceWithSymbolDefinitions {
                    reference: Identifier {
                        name: String::from("_barrier_cost"),
                        range: FileRange {
                            path: String::from("graph.py"),
                            start: Position {
                                line: 39,
                                character: 28,
                            },
                            end: Position {
                                line: 39,
                                character: 41,
                            },
                        },
                        kind: Some(String::from("function-call")),
                    },
                    definitions: vec![Symbol {
                        name: String::from("_barrier_cost"),
                        kind: String::from("function"),
                        identifier_position: FilePosition {
                            path: String::from("graph.py"),
                            position: Position {
                                line: 26,
                                character: 8,
                            },
                        },
                        range: FileRange {
                            path: String::from("graph.py"),
                            start: Position {
                                line: 26,
                                character: 0,
                            },
                            end: Position {
                                line: 31,
                                character: 16,
                            },
                        },
                    }],
                },
                ReferenceWithSymbolDefinitions {
                    reference: Identifier {
                        name: String::from("_distance_cost"),
                        range: FileRange {
                            path: String::from("graph.py"),
                            start: Position {
                                line: 40,
                                character: 29,
                            },
                            end: Position {
                                line: 40,
                                character: 43,
                            },
                        },
                        kind: Some(String::from("function-call")),
                    },
                    definitions: vec![Symbol {
                        name: String::from("_distance_cost"),
                        kind: String::from("function"),
                        identifier_position: FilePosition {
                            path: String::from("graph.py"),
                            position: Position {
                                line: 33,
                                character: 8,
                            },
                        },
                        range: FileRange {
                            path: String::from("graph.py"),
                            start: Position {
                                line: 33,
                                character: 0,
                            },
                            end: Position {
                                line: 35,
                                character: 50,
                            },
                        },
                    }],
                },
                ReferenceWithSymbolDefinitions {
                    reference: Identifier {
                        name: String::from("cost_function"),
                        range: FileRange {
                            path: String::from("graph.py"),
                            start: Position {
                                line: 65,
                                character: 15,
                            },
                            end: Position {
                                line: 65,
                                character: 28,
                            },
                        },
                        kind: Some(String::from("function-call")),
                    },
                    definitions: vec![
                        Symbol {
                            name: String::from("_barrier_cost"),
                            kind: String::from("function"),
                            identifier_position: FilePosition {
                                path: String::from("graph.py"),
                                position: Position {
                                    line: 26,
                                    character: 8,
                                },
                            },
                            range: FileRange {
                                path: String::from("graph.py"),
                                start: Position {
                                    line: 26,
                                    character: 0,
                                },
                                end: Position {
                                    line: 31,
                                    character: 16,
                                },
                            },
                        },
                        Symbol {
                            name: String::from("_distance_cost"),
                            kind: String::from("function"),
                            identifier_position: FilePosition {
                                path: String::from("graph.py"),
                                position: Position {
                                    line: 33,
                                    character: 8,
                                },
                            },
                            range: FileRange {
                                path: String::from("graph.py"),
                                start: Position {
                                    line: 33,
                                    character: 0,
                                },
                                end: Position {
                                    line: 35,
                                    character: 50,
                                },
                            },
                        },
                        Symbol {
                            name: String::from("_combined_cost"),
                            kind: String::from("function"),
                            identifier_position: FilePosition {
                                path: String::from("graph.py"),
                                position: Position {
                                    line: 37,
                                    character: 8,
                                },
                            },
                            range: FileRange {
                                path: String::from("graph.py"),
                                start: Position {
                                    line: 37,
                                    character: 0,
                                },
                                end: Position {
                                    line: 41,
                                    character: 43,
                                },
                            },
                        },
                    ],
                },
                ReferenceWithSymbolDefinitions {
                    reference: Identifier {
                        name: String::from("log_execution_time"),
                        range: FileRange {
                            path: String::from("graph.py"),
                            start: Position {
                                line: 67,
                                character: 5,
                            },
                            end: Position {
                                line: 67,
                                character: 23,
                            },
                        },
                        kind: Some(String::from("decorator")),
                    },
                    definitions: vec![Symbol {
                        name: String::from("log_execution_time"),
                        kind: String::from("function"),
                        identifier_position: FilePosition {
                            path: String::from("decorators.py"),
                            position: Position {
                                line: 3,
                                character: 4,
                            },
                        },
                        range: FileRange {
                            path: String::from("decorators.py"),
                            start: Position {
                                line: 3,
                                character: 0,
                            },
                            end: Position {
                                line: 11,
                                character: 18,
                            },
                        },
                    }],
                },
                ReferenceWithSymbolDefinitions {
                    reference: Identifier {
                        name: String::from("log_execution_time"),
                        range: FileRange {
                            path: String::from("graph.py"),
                            start: Position {
                                line: 75,
                                character: 5,
                            },
                            end: Position {
                                line: 75,
                                character: 23,
                            },
                        },
                        kind: Some(String::from("decorator")),
                    },
                    definitions: vec![Symbol {
                        name: String::from("log_execution_time"),
                        kind: String::from("function"),
                        identifier_position: FilePosition {
                            path: String::from("decorators.py"),
                            position: Position {
                                line: 3,
                                character: 4,
                            },
                        },
                        range: FileRange {
                            path: String::from("decorators.py"),
                            start: Position {
                                line: 3,
                                character: 0,
                            },
                            end: Position {
                                line: 11,
                                character: 18,
                            },
                        },
                    }],
                },
                ReferenceWithSymbolDefinitions {
                    reference: Identifier {
                        name: String::from("move_cost"),
                        range: FileRange {
                            path: String::from("graph.py"),
                            start: Position {
                                line: 86,
                                character: 20,
                            },
                            end: Position {
                                line: 86,
                                character: 29,
                            },
                        },
                        kind: Some(String::from("function-call")),
                    },
                    definitions: vec![Symbol {
                        name: String::from("move_cost"),
                        kind: String::from("function"),
                        identifier_position: FilePosition {
                            path: String::from("graph.py"),
                            position: Position {
                                line: 43,
                                character: 8,
                            },
                        },
                        range: FileRange {
                            path: String::from("graph.py"),
                            start: Position {
                                line: 43,
                                character: 0,
                            },
                            end: Position {
                                line: 65,
                                character: 34,
                            },
                        },
                    }],
                },
            ],
            external_symbols: vec![
                Identifier {
                    name: String::from("append"),
                    range: FileRange {
                        path: String::from("graph.py"),
                        start: Position {
                            line: 15,
                            character: 23,
                        },
                        end: Position {
                            line: 15,
                            character: 29,
                        },
                    },
                    kind: Some(String::from("function-call")),
                },
                Identifier {
                    name: String::from("property"),
                    range: FileRange {
                        path: String::from("graph.py"),
                        start: Position {
                            line: 22,
                            character: 5,
                        },
                        end: Position {
                            line: 22,
                            character: 13,
                        },
                    },
                    kind: Some(String::from("decorator")),
                },
                Identifier {
                    name: String::from("abs"),
                    range: FileRange {
                        path: String::from("graph.py"),
                        start: Position {
                            line: 35,
                            character: 15,
                        },
                        end: Position {
                            line: 35,
                            character: 18,
                        },
                    },
                    kind: Some(String::from("function-call")),
                },
                Identifier {
                    name: String::from("abs"),
                    range: FileRange {
                        path: String::from("graph.py"),
                        start: Position {
                            line: 35,
                            character: 34,
                        },
                        end: Position {
                            line: 35,
                            character: 37,
                        },
                    },
                    kind: Some(String::from("function-call")),
                },
                Identifier {
                    name: String::from("ValueError"),
                    range: FileRange {
                        path: String::from("graph.py"),
                        start: Position {
                            line: 63,
                            character: 18,
                        },
                        end: Position {
                            line: 63,
                            character: 28,
                        },
                    },
                    kind: Some(String::from("function-call")),
                },
                Identifier {
                    name: String::from("abs"),
                    range: FileRange {
                        path: String::from("graph.py"),
                        start: Position {
                            line: 71,
                            character: 13,
                        },
                        end: Position {
                            line: 71,
                            character: 16,
                        },
                    },
                    kind: Some(String::from("function-call")),
                },
                Identifier {
                    name: String::from("abs"),
                    range: FileRange {
                        path: String::from("graph.py"),
                        start: Position {
                            line: 72,
                            character: 13,
                        },
                        end: Position {
                            line: 72,
                            character: 16,
                        },
                    },
                    kind: Some(String::from("function-call")),
                },
                Identifier {
                    name: String::from("min"),
                    range: FileRange {
                        path: String::from("graph.py"),
                        start: Position {
                            line: 73,
                            character: 46,
                        },
                        end: Position {
                            line: 73,
                            character: 49,
                        },
                    },
                    kind: Some(String::from("function-call")),
                },
                Identifier {
                    name: String::from("append"),
                    range: FileRange {
                        path: String::from("graph.py"),
                        start: Position {
                            line: 87,
                            character: 18,
                        },
                        end: Position {
                            line: 87,
                            character: 24,
                        },
                    },
                    kind: Some(String::from("function-call")),
                },
            ],
            not_found: vec![],
        };

        assert_eq!(referenced_symbols_response, expected_response);
        Ok(())
    }
}
