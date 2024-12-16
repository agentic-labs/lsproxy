use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use log::{error, info};
use lsp_types::{
    CallHierarchyIncomingCall, CallHierarchyIncomingCallsParams, CallHierarchyItem,
    CallHierarchyOutgoingCall, CallHierarchyOutgoingCallsParams, CallHierarchyPrepareParams,
    Position as LspPosition, TextDocumentPositionParams,
};
use serde::{Deserialize, Serialize};

use crate::api_types::{
    CallHierarchyResponse, CallLocation, CallReference, ErrorResponse, FilePosition,
    GetCallHierarchyRequest, Position,
};
use crate::lsp::manager::{LspManagerError, Manager};
use crate::utils::file_utils::uri_to_relative_path_string;
use crate::AppState;

/// Get call hierarchy for a function or method
///
/// Returns incoming and outgoing calls for the function at the given position.
/// The input position should point to the identifier of the function you want to analyze.
#[utoipa::path(
    post,
    path = "/symbol/call-hierarchy",
    tag = "symbol",
    request_body = GetCallHierarchyRequest,
    responses(
        (status = 200, description = "Call hierarchy retrieved successfully", body = CallHierarchyResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_call_hierarchy(
    data: Data<AppState>,
    info: Json<GetCallHierarchyRequest>,
) -> HttpResponse {
    info!(
        "Received call hierarchy request for file: {}, line: {}, character: {}",
        info.identifier_position.path,
        info.identifier_position.position.line,
        info.identifier_position.position.character
    );
    let manager = data.manager.lock().unwrap();

    // Prepare call hierarchy
    let prepare_result = manager
        .prepare_call_hierarchy(
            &info.identifier_position.path,
            LspPosition {
                line: info.identifier_position.position.line,
                character: info.identifier_position.position.character,
            },
        )
        .await;

    match prepare_result {
        Ok(items) if !items.is_empty() => {
            let item = &items[0]; // Take first item as per gopls implementation

            // Get incoming calls
            let incoming_result = manager.incoming_calls(item).await;
            let outgoing_result = manager.outgoing_calls(item).await;

            match (incoming_result, outgoing_result) {
                (Ok(incoming), Ok(outgoing)) => {
                    let response = CallHierarchyResponse {
                        incoming_calls: incoming
                            .into_iter()
                            .map(|call| CallReference {
                                from: convert_hierarchy_item(&call.from),
                                ranges: call
                                    .from_ranges
                                    .into_iter()
                                    .map(|r| Position {
                                        line: r.start.line,
                                        character: r.start.character,
                                    })
                                    .collect(),
                            })
                            .collect(),
                        outgoing_calls: outgoing
                            .into_iter()
                            .map(|call| CallReference {
                                from: convert_hierarchy_item(&call.to),
                                ranges: call
                                    .from_ranges
                                    .into_iter()
                                    .map(|r| Position {
                                        line: r.start.line,
                                        character: r.start.character,
                                    })
                                    .collect(),
                            })
                            .collect(),
                        item: convert_hierarchy_item(item),
                    };

                    HttpResponse::Ok().json(response)
                }
                (Err(e), _) | (_, Err(e)) => {
                    error!("Failed to get call hierarchy details: {}", e);
                    HttpResponse::InternalServerError().json(ErrorResponse {
                        error: format!("Failed to get call hierarchy details: {}", e),
                    })
                }
            }
        }
        Ok(_) => HttpResponse::BadRequest().json(ErrorResponse {
            error: "No function found at the given position".to_string(),
        }),
        Err(e) => {
            error!("Failed to prepare call hierarchy: {}", e);
            match e {
                LspManagerError::FileNotFound(path) => HttpResponse::BadRequest().json(ErrorResponse {
                    error: format!("File not found: {}", path),
                }),
                LspManagerError::LspClientNotFound(lang) => {
                    HttpResponse::InternalServerError().json(ErrorResponse {
                        error: format!("LSP client not found for {:?}", lang),
                    })
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
    }
}

fn convert_hierarchy_item(item: &CallHierarchyItem) -> CallLocation {
    CallLocation {
        path: uri_to_relative_path_string(&item.uri),
        name: item.name.clone(),
        range_start: Position {
            line: item.range.start.line,
            character: item.range.start.character,
        },
        range_end: Position {
            line: item.range.end.line,
            character: item.range.end.character,
        },
        selection_range_start: Position {
            line: item.selection_range.start.line,
            character: item.selection_range.start.character,
        },
        selection_range_end: Position {
            line: item.selection_range.end.line,
            character: item.selection_range.end.character,
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use actix_web::http::StatusCode;
    use tokio::time::{sleep, Duration};

    use crate::initialize_app_state;
    use crate::test_utils::{python_sample_path, rust_sample_path, TestContext};

    #[tokio::test]
    async fn test_python_call_hierarchy() -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup(&python_sample_path(), false).await?;
        let state = initialize_app_state().await?;

        let mock_request = Json(GetCallHierarchyRequest {
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 1,
                    character: 6,
                },
            },
        });

        let response = get_call_hierarchy(state, mock_request).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json"
        );

        // Check the body
        let body = response.into_body();
        let bytes = actix_web::body::to_bytes(body).await.unwrap();
        let hierarchy_response: CallHierarchyResponse = serde_json::from_slice(&bytes).unwrap();

        // Basic validation - actual values will depend on the test files
        assert!(hierarchy_response.item.name.contains("Graph"));
        Ok(())
    }

    #[tokio::test]
    async fn test_rust_call_hierarchy() -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup(&rust_sample_path(), false).await?;
        let state = initialize_app_state().await?;

        let mock_request = Json(GetCallHierarchyRequest {
            identifier_position: FilePosition {
                path: String::from("src/node.rs"),
                position: Position {
                    line: 3,
                    character: 11,
                },
            },
        });

        sleep(Duration::from_secs(5)).await;

        let response = get_call_hierarchy(state, mock_request).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json"
        );

        // Check the body
        let body = response.into_body();
        let bytes = actix_web::body::to_bytes(body).await.unwrap();
        let hierarchy_response: CallHierarchyResponse = serde_json::from_slice(&bytes).unwrap();

        // Basic validation - actual values will depend on the test files
        assert!(hierarchy_response.item.name.contains("Node"));
        Ok(())
    }
}