use crate::api_types::{CodeContext, ErrorResponse, FileRange, Position};
use crate::handlers::utils;
use crate::lsp::manager::{LspManagerError, Manager};
use crate::utils::file_utils::uri_to_relative_path_string;
use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use log::{error, info, warn};

use crate::api_types::{DefinitionResponse, GetDefinitionRequest};
use crate::AppState;
use lsp_types::{GotoDefinitionResponse, Location, Position as LspPosition, Range};
/// Get the definition of a symbol at a specific position in a file
///
/// Returns the location of the definition for the symbol at the given position.
///
/// The input position should point inside the symbol's identifier, e.g.
///
/// The returned position points to the identifier of the symbol, and the file_path from workspace root
///
/// e.g. for the definition of `User` on line 5 of `src/main.py` with the code:
/// ```
/// 0: class User:
/// output___^
/// 1:     def __init__(self, name, age):
/// 2:         self.name = name
/// 3:         self.age = age
/// 4:
/// 5: user = User("John", 30)
/// input_____^^^^
/// ```
#[utoipa::path(
    post,
    path = "/symbol/find-definition",
    tag = "symbol",
    request_body = GetDefinitionRequest,
    responses(
        (status = 200, description = "Definition retrieved successfully", body = DefinitionResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn find_definition(
    data: Data<AppState>,
    info: Json<GetDefinitionRequest>,
) -> HttpResponse {
    info!(
        "Received definition request for file: {}, line: {}, character: {}",
        info.position.path, info.position.position.line, info.position.position.character
    );

    let manager = match data.manager.lock() {
        Ok(manager) => manager,
        Err(e) => {
            error!("Failed to lock manager: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to lock manager: {}", e),
            });
        }
    };
    let file_identifiers = match manager.get_file_identifiers(&info.position.path).await {
        Ok(identifiers) => identifiers,
        Err(e) => {
            error!("Failed to get file identifiers: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to get file identifiers: {}", e),
            });
        }
    };
    let identifier =
        match utils::find_identifier_at_position(file_identifiers, &info.position).await {
            Ok(identifier) => identifier,
            Err(e) => {
                error!("Failed to find definition from position: {:?}", e);
                return HttpResponse::BadRequest().json(ErrorResponse {
                    error: format!("Failed to find definition from position: {}", e),
                });
            }
        };

    let definitions = match manager
        .find_definition(
            &info.position.path,
            LspPosition {
                line: info.position.position.line,
                character: info.position.position.character,
            },
        )
        .await
    {
        Ok(definitions) => definitions,
        Err(e) => {
            error!("Definition error: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Definition retrieval failed: {}", e),
            });
        }
    };

    let source_code_context = if info.include_source_code {
        match fetch_definition_source_code(&manager, &definitions).await {
            Ok(context) => Some(context),
            Err(e) => {
                error!("Failed to fetch definition source code: {:?}", e);
                None
            }
        }
    } else {
        None
    };

    HttpResponse::Ok().json(DefinitionResponse {
        raw_response: if info.include_raw_response {
            Some(serde_json::to_value(&definitions).unwrap())
        } else {
            None
        },
        definitions: match &definitions {
            GotoDefinitionResponse::Scalar(location) => vec![location.clone().into()],
            GotoDefinitionResponse::Array(locations) => {
                locations.iter().map(|l| l.clone().into()).collect()
            }
            GotoDefinitionResponse::Link(links) => links.iter().map(|l| l.clone().into()).collect(),
        },
        source_code_context,
        selected_identifier: identifier,
    })
}

async fn fetch_definition_source_code(
    manager: &Manager,
    definitions_response: &GotoDefinitionResponse,
) -> Result<Vec<CodeContext>, LspManagerError> {
    let mut code_contexts = Vec::new();
    let definitions: &Vec<Location> = match definitions_response {
        GotoDefinitionResponse::Scalar(definition) => &vec![definition.clone()],
        GotoDefinitionResponse::Array(definitions) => definitions,
        GotoDefinitionResponse::Link(links) => &links
            .iter()
            .map(|link| Location::new(link.target_uri.clone(), link.target_range))
            .collect::<Vec<Location>>(),
    };

    for definition in definitions {
        let relative_path = uri_to_relative_path_string(&definition.uri);
        let file_symbols = manager.definitions_in_file_ast_grep(&relative_path).await?;
        let symbol = file_symbols.iter().find(|s| {
            s.range.start.line as u32 == definition.range.start.line
                && s.range.start.column as u32 == definition.range.start.character
        });

        let source_code_context = match symbol {
            Some(ast_grep_match) => CodeContext {
                range: FileRange {
                    path: relative_path,
                    start: Position {
                        line: ast_grep_match.get_range().start.line as u32,
                        character: ast_grep_match.get_range().start.column as u32,
                    },
                    end: Position {
                        line: ast_grep_match.get_range().end.line as u32,
                        character: ast_grep_match.get_range().end.column as u32,
                    },
                },
                source_code: ast_grep_match.get_source_code(),
            },
            None => {
                warn!("Symbol not found for definition: {:?}", definition);
                warn!("No exact match in file symbols (likely filtered out). Returning an approximate range instead.");
                let range = Range {
                    start: LspPosition {
                        line: definition.range.start.line.saturating_sub(3),
                        character: 0,
                    },
                    end: LspPosition {
                        line: definition.range.end.line as u32 + 3,
                        character: 0,
                    },
                };
                let source_code = manager
                    .read_source_code(&relative_path, Some(range))
                    .await?;
                CodeContext {
                    range: FileRange {
                        path: relative_path,
                        start: Position {
                            line: definition.range.start.line.saturating_sub(3),
                            character: 0,
                        },
                        end: Position {
                            line: definition.range.end.line as u32 + 3,
                            character: 0,
                        },
                    },
                    source_code,
                }
            }
        };

        code_contexts.push(source_code_context);
    }
    Ok(code_contexts)
}

#[cfg(test)]
mod test {
    use super::*;

    use actix_web::http::StatusCode;

    use crate::api_types::{FilePosition, Identifier, Position};
    use crate::initialize_app_state;
    use crate::test_utils::{python_sample_path, TestContext};

    #[tokio::test]
    async fn test_python_definition() -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup(&python_sample_path(), false).await?;
        let state = initialize_app_state().await?;

        let mock_request = Json(GetDefinitionRequest {
            position: FilePosition {
                path: String::from("main.py"),
                position: Position {
                    line: 1,
                    character: 18,
                },
            },
            include_source_code: true,
            include_raw_response: false,
        });

        let response = find_definition(state, mock_request).await;

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "{}",
            format!("{:?}", response.body())
        );
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json"
        );

        let body = response.into_body();
        let bytes = actix_web::body::to_bytes(body).await.unwrap();
        let definition_response: DefinitionResponse = serde_json::from_slice(&bytes).unwrap();

        let expected_response = DefinitionResponse {
            raw_response: None,
            definitions: vec![FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 1,
                    character: 6,
                },
            }],
            source_code_context: Some(vec![CodeContext {
                range: FileRange {
                    path: String::from("graph.py"),
                    start: Position {
                        line: 1,
                        character: 0,
                    },
                    end: Position {
                        line: 60,
                        character: 40,
                    },
                },
                source_code: String::from("class AStarGraph(object):\n    # Define a class board like grid with two barriers\n\n    def __init__(self):\n        self.barriers = []\n        self.barriers.append(\n            [\n                (2, 4),\n                (2, 5),\n                (2, 6),\n                (3, 6),\n                (4, 6),\n                (5, 6),\n                (5, 5),\n                (5, 4),\n                (5, 3),\n                (5, 2),\n                (4, 2),\n                (3, 2),\n            ]\n        )\n\n    @property\n    def barriers(self):\n        return self.barriers\n\n    def heuristic(self, start, goal):\n        # Use Chebyshev distance heuristic if we can move one square either\n        # adjacent or diagonal\n        D = 1\n        D2 = 1\n        dx = abs(start[0] - goal[0])\n        dy = abs(start[1] - goal[1])\n        return D * (dx + dy) + (D2 - 2 * D) * min(dx, dy)\n\n    def get_vertex_neighbours(self, pos):\n        n = []\n        # Moves allow link a chess king\n        for dx, dy in [\n            (1, 0),\n            (-1, 0),\n            (0, 1),\n            (0, -1),\n            (1, 1),\n            (-1, 1),\n            (1, -1),\n            (-1, -1),\n        ]:\n            x2 = pos[0] + dx\n            y2 = pos[1] + dy\n            if x2 < 0 or x2 > 7 or y2 < 0 or y2 > 7:\n                continue\n            n.append((x2, y2))\n        return n\n\n    def move_cost(self, a, b):\n        for barrier in self.barriers:\n            if b in barrier:\n                return 100  # Extremely high cost to enter barrier squares\n        return 1  # Normal movement cost"),
            }]),
            selected_identifier: Identifier {
                name: String::from("AStarGraph"),
                range: FileRange {
                    path: String::from("main.py"),
                    start: Position {
                        line: 1,
                        character: 18,
                    },
                    end: Position {
                        line: 1,
                        character: 28,
                    },
                },
            },
        };

        assert_eq!(definition_response, expected_response);
        Ok(())
    }

    #[tokio::test]
    async fn test_invalid_position() -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup(&python_sample_path(), false).await?;
        let state = initialize_app_state().await?;

        let mock_request = Json(GetDefinitionRequest {
            position: FilePosition {
                path: String::from("main.py"),
                position: Position {
                    line: 0, // Invalid line number
                    character: 999,
                },
            },
            include_source_code: false,
            include_raw_response: false,
        });

        let response = find_definition(state, mock_request).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json"
        );

        let body = response.into_body();
        let bytes = actix_web::body::to_bytes(body).await.unwrap();
        let error_response: ErrorResponse = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(
            error_response.error,
            "Failed to find definition from position: No identifier found at position. Closest matches: [Identifier { name: \"plt\", range: FileRange { path: \"main.py\", start: Position { line: 0, character: 28 }, end: Position { line: 0, character: 31 } } }, Identifier { name: \"pyplot\", range: FileRange { path: \"main.py\", start: Position { line: 0, character: 18 }, end: Position { line: 0, character: 24 } } }, Identifier { name: \"matplotlib\", range: FileRange { path: \"main.py\", start: Position { line: 0, character: 7 }, end: Position { line: 0, character: 17 } } }]"
        );
        Ok(())
    }
}
