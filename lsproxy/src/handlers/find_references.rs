use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use log::{error, info};
use lsp_types::{Location, Position as LspPosition};

use crate::api_types::{
    CodeContext, ErrorResponse, FilePosition, FileRange, GetReferencesRequest, Position,
    ReferencesResponse,
};
use crate::handlers::utils;
use crate::lsp::manager::{LspManagerError, Manager};
use crate::utils::file_utils::uri_to_relative_path_string;
use crate::AppState;

/// Find all references to a symbol
///
/// The input position should point to the identifier of the symbol you want to get the references for.
///
/// Returns a list of locations where the symbol at the given position is referenced.
///
/// The returned positions point to the start of the reference identifier.
///
/// e.g. for `User` on line 0 of `src/main.py`:
/// ```
///  0: class User:
///  input____^^^^
///  1:     def __init__(self, name, age):
///  2:         self.name = name
///  3:         self.age = age
///  4:
///  5: user = User("John", 30)
///  output____^
/// ```
#[utoipa::path(
    post,
    path = "/symbol/find-references",
    tag = "symbol",
    request_body = GetReferencesRequest,
    responses(
        (status = 200, description = "References retrieved successfully", body = ReferencesResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn find_references(
    data: Data<AppState>,
    info: Json<GetReferencesRequest>,
) -> HttpResponse {
    info!(
        "Received references request for file: {}, line: {}, character: {}",
        info.identifier_position.path,
        info.identifier_position.position.line,
        info.identifier_position.position.character
    );

    let manager = match data.manager.lock() {
        Ok(guard) => guard,
        Err(e) => {
            error!("Failed to acquire manager lock: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Internal server error: failed to acquire lock".to_string(),
            });
        }
    };

    let file_identifiers = match manager
        .get_file_identifiers(&info.identifier_position.path)
        .await
    {
        Ok(identifiers) => identifiers,
        Err(e) => {
            error!("Failed to get file identifiers: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to get file identifiers: {}", e),
            });
        }
    };

    let selected_identifier =
        match utils::find_identifier_at_position(file_identifiers, &info.identifier_position).await
        {
            Ok(identifier) => identifier,
            Err(e) => {
                error!("Failed to find references from position: {:?}", e);
                return HttpResponse::BadRequest().json(ErrorResponse {
                    error: format!("Failed to find references from position: {}", e),
                });
            }
        };

    let references_result = find_and_filter_references(&manager, &info.identifier_position).await;
    let code_contexts_result = get_code_contexts(
        &manager,
        &references_result,
        info.include_code_context_lines,
    )
    .await;

    match (references_result, code_contexts_result) {
        (Ok(references), Ok(code_contexts)) => {
            let raw_response = if info.include_raw_response {
                match serde_json::to_value(&references) {
                    Ok(value) => Some(value),
                    Err(e) => {
                        error!("Failed to serialize raw response: {}", e);
                        None
                    }
                }
            } else {
                None
            };

            let response = ReferencesResponse {
                raw_response,
                references: references
                    .into_iter()
                    .map(|loc| FilePosition {
                        path: uri_to_relative_path_string(&loc.uri),
                        position: Position {
                            line: loc.range.start.line,
                            character: loc.range.start.character,
                        },
                    })
                    .collect(),
                context: code_contexts,
                selected_identifier,
            };
            HttpResponse::Ok().json(response)
        }
        (Err(e), _) => handle_lsp_error(e),
        (_, Err(e)) => {
            error!("Failed to fetch code context: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to fetch code context: {}", e),
            })
        }
    }
}

async fn find_and_filter_references(
    manager: &Manager,
    position: &FilePosition,
) -> Result<Vec<Location>, LspManagerError> {
    let references = manager
        .find_references(
            &position.path,
            LspPosition {
                line: position.position.line,
                character: position.position.character,
            },
        )
        .await?;

    let files = manager.list_files().await?;
    let mut filtered_refs: Vec<_> = references
        .into_iter()
        .filter(|reference| {
            let path = uri_to_relative_path_string(&reference.uri);
            files.contains(&path)
        })
        .collect();

    filtered_refs.sort_by(|a, b| {
        let uri_cmp = a.uri.to_string().cmp(&b.uri.to_string());
        if uri_cmp.is_eq() {
            a.range.start.line.cmp(&b.range.start.line)
        } else {
            uri_cmp
        }
    });

    Ok(filtered_refs)
}

async fn get_code_contexts(
    manager: &Manager,
    references_result: &Result<Vec<Location>, LspManagerError>,
    context_lines: Option<u32>,
) -> Result<Option<Vec<CodeContext>>, LspManagerError> {
    match (references_result, context_lines) {
        (Ok(refs), Some(lines)) => fetch_code_context(manager, refs.clone(), lines)
            .await
            .map(Some),
        _ => Ok(None),
    }
}

fn handle_lsp_error(e: LspManagerError) -> HttpResponse {
    error!("Failed to get references: {}", e);
    match e {
        LspManagerError::FileNotFound(path) => HttpResponse::BadRequest().json(ErrorResponse {
            error: format!("File not found: {}", path),
        }),
        LspManagerError::LspClientNotFound(lang) => {
            HttpResponse::InternalServerError().body(format!("LSP client not found for {:?}", lang))
        }
        LspManagerError::InternalError(msg) => {
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Internal error: {}", msg),
            })
        }
        LspManagerError::UnsupportedFileType(path) => {
            HttpResponse::BadRequest().json(ErrorResponse {
                error: format!("Unsupported file type: {}", path),
            })
        }
    }
}

async fn fetch_code_context(
    manager: &Manager,
    references: Vec<Location>,
    context_lines: u32,
) -> Result<Vec<CodeContext>, LspManagerError> {
    let mut code_contexts = Vec::new();
    for reference in references {
        let range = lsp_types::Range {
            start: LspPosition {
                line: reference.range.start.line.saturating_sub(context_lines),
                character: 0,
            },
            end: LspPosition {
                line: reference.range.end.line.saturating_add(context_lines),
                character: 0,
            },
        };
        match manager
            .read_source_code(&uri_to_relative_path_string(&reference.uri), Some(range))
            .await
        {
            Ok(source_code) => {
                code_contexts.push(CodeContext {
                    source_code,
                    range: FileRange {
                        path: uri_to_relative_path_string(&reference.uri),
                        start: Position {
                            line: range.start.line,
                            character: 0,
                        },
                        end: Position {
                            line: range.end.line,
                            character: 0,
                        },
                    },
                });
            }
            Err(e) => return Err(e),
        }
    }
    Ok(code_contexts)
}

#[cfg(test)]
mod test {
    use super::*;

    use actix_web::http::StatusCode;
    use tokio::time::{sleep, Duration};

    use crate::api_types::{FilePosition, Identifier, Position};
    use crate::initialize_app_state;
    use crate::test_utils::{python_sample_path, rust_sample_path, TestContext};

    #[tokio::test]
    async fn test_python_references() -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup(&python_sample_path(), false).await?;
        let state = initialize_app_state().await?;

        let mock_request = Json(GetReferencesRequest {
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 12,
                    character: 6,
                },
            },
            include_code_context_lines: None,
            include_raw_response: false,
        });

        let response = find_references(state, mock_request).await;

        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response
            .headers()
            .get("content-type")
            .ok_or("Missing content-type header")?
            .to_str()?;
        assert_eq!(content_type, "application/json");

        // Check the body
        let body = response.into_body();
        let bytes = actix_web::body::to_bytes(body).await?;
        let reference_response: ReferencesResponse = serde_json::from_slice(&bytes)?;

        let expected_response = ReferencesResponse {
            raw_response: None,
            references: vec![
                FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 12,
                        character: 6,
                    },
                },
                FilePosition {
                    path: String::from("main.py"),
                    position: Position {
                        line: 1,
                        character: 18,
                    },
                },
                FilePosition {
                    path: String::from("main.py"),
                    position: Position {
                        line: 6,
                        character: 27,
                    },
                },
                FilePosition {
                    path: String::from("main.py"),
                    position: Position {
                        line: 15,
                        character: 12,
                    },
                },
                FilePosition {
                    path: String::from("search.py"),
                    position: Position {
                        line: 1,
                        character: 18,
                    },
                },
                FilePosition {
                    path: String::from("search.py"),
                    position: Position {
                        line: 5,
                        character: 41,
                    },
                },
                FilePosition {
                    path: String::from("search.py"),
                    position: Position {
                        line: 16,
                        character: 37,
                    },
                },
            ],
            context: None,
            selected_identifier: Identifier {
                name: String::from("AStarGraph"),
                range: FileRange {
                    path: String::from("graph.py"),
                    start: Position {
                        line: 12,
                        character: 6,
                    },
                    end: Position {
                        line: 12,
                        character: 16,
                    },
                },
            },
        };

        assert_eq!(reference_response, expected_response);
        Ok(())
    }

    #[tokio::test]
    async fn test_rust_references() -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup(&rust_sample_path(), false).await?;
        let state = initialize_app_state().await?;

        let mock_request = Json(GetReferencesRequest {
            identifier_position: FilePosition {
                path: String::from("src/node.rs"),
                position: Position {
                    line: 3,
                    character: 11,
                },
            },
            include_code_context_lines: None,
            include_raw_response: false,
        });

        sleep(Duration::from_secs(5)).await;

        let response = find_references(state, mock_request).await;

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "{}",
            format!("{:?}", response.body())
        );
        let content_type = response
            .headers()
            .get("content-type")
            .ok_or("Missing content-type header")?
            .to_str()?;
        assert_eq!(content_type, "application/json");

        // Check the body
        let body = response.into_body();
        let bytes = actix_web::body::to_bytes(body).await?;
        let reference_response: ReferencesResponse = serde_json::from_slice(&bytes)?;

        let expected_response = ReferencesResponse {
            raw_response: None,
            references: vec![
                FilePosition {
                    path: String::from("src/astar.rs"),
                    position: Position {
                        line: 1,
                        character: 17,
                    },
                },
                FilePosition {
                    path: String::from("src/astar.rs"),
                    position: Position {
                        line: 6,
                        character: 14,
                    },
                },
                FilePosition {
                    path: String::from("src/astar.rs"),
                    position: Position {
                        line: 7,
                        character: 16,
                    },
                },
                FilePosition {
                    path: String::from("src/astar.rs"),
                    position: Position {
                        line: 59,
                        character: 32,
                    },
                },
                FilePosition {
                    path: String::from("src/astar.rs"),
                    position: Position {
                        line: 76,
                        character: 35,
                    },
                },
                FilePosition {
                    path: String::from("src/astar.rs"),
                    position: Position {
                        line: 93,
                        character: 23,
                    },
                },
                FilePosition {
                    path: String::from("src/node.rs"),
                    position: Position {
                        line: 3,
                        character: 11,
                    },
                },
                FilePosition {
                    path: String::from("src/node.rs"),
                    position: Position {
                        line: 10,
                        character: 20,
                    },
                },
                FilePosition {
                    path: String::from("src/node.rs"),
                    position: Position {
                        line: 11,
                        character: 34,
                    },
                },
            ],
            context: None,
            selected_identifier: reference_response.selected_identifier.clone(), // We can't predict this value
        };

        assert_eq!(expected_response, reference_response);
        Ok(())
    }

    #[tokio::test]
    async fn test_invalid_position() -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup(&python_sample_path(), false).await?;
        let state = initialize_app_state().await?;

        let mock_request = Json(GetReferencesRequest {
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 999, // Invalid line number
                    character: 0,
                },
            },
            include_code_context_lines: None,
            include_raw_response: false,
        });

        let response = find_references(state, mock_request).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = response.into_body();
        let bytes = actix_web::body::to_bytes(body).await?;
        let error_response: ErrorResponse = serde_json::from_slice(&bytes)?;
        assert_eq!(
            error_response.error,
            "Failed to find references from position: No identifier found at position. Closest matches: [Identifier { name: \"n\", range: FileRange { path: \"graph.py\", start: Position { line: 88, character: 15 }, end: Position { line: 88, character: 16 } } }, Identifier { name: \"n\", range: FileRange { path: \"graph.py\", start: Position { line: 87, character: 16 }, end: Position { line: 87, character: 17 } } }, Identifier { name: \"append\", range: FileRange { path: \"graph.py\", start: Position { line: 87, character: 18 }, end: Position { line: 87, character: 24 } } }]"
        );

        Ok(())
    }
}
