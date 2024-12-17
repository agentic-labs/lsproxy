use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use log::{error, info};
use lsp_types::{
    CallHierarchyItem,    
    Position as LspPosition,
};


use crate::api_types::{
    CallHierarchyItemDetails, CallHierarchyResponse, CallLocation, CallReference, ErrorResponse, FilePosition, GetCallHierarchyRequest, Position
};
use crate::lsp::manager::LspManagerError;
use crate::utils::file_utils::uri_to_relative_path_string;
use crate::AppState;

/// Get call hierarchy for a function or method
///
/// Returns incoming and outgoing calls for the function at the given position.
/// The input position should point to the identifier of the function you want to analyze.
///
/// Supports two modes of operation:
/// 1. LSP server-based analysis (default, use_manual_hierarchy=false)
/// 2. Manual AST-based analysis using tree-sitter (use_manual_hierarchy=true)
///
/// The manual mode may provide better results for some languages or when the LSP server
/// doesn't support call hierarchy functionality.
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
            info.use_manual_hierarchy,
        )
        .await;

    match prepare_result {
        Ok(items) if !items.is_empty() => {
            let mut hierarchies = Vec::new();

            for item in items {
                // Get incoming and outgoing calls for each item
                let incoming_result = manager.incoming_calls(&item).await;
                let outgoing_result = manager.outgoing_calls(&item, info.use_manual_hierarchy).await;

                match (incoming_result, outgoing_result) {
                    (Ok(incoming), Ok(outgoing)) => {
                        let hierarchy_item = CallHierarchyItemDetails {
                            item: convert_hierarchy_item(&item),
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
                        };
                        hierarchies.push(hierarchy_item);
                    }
                    (Err(e), _) | (_, Err(e)) => {
                        error!("Failed to get call hierarchy details for item: {}", e);
                        // Continue with next item instead of failing completely
                        continue;
                    }
                }
            }

            if hierarchies.is_empty() {
                HttpResponse::BadRequest().json(ErrorResponse {
                    error: "Failed to get call hierarchy details for any items".to_string(),
                })
            } else {
                HttpResponse::Ok().json(CallHierarchyResponse { items: hierarchies })
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
    use crate::test_utils::{python_sample_path, typescript_sample_path, TestContext};

    #[tokio::test]
    async fn test_python_call_hierarchy() -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup(&python_sample_path(), false).await?;
        let state = initialize_app_state().await?;

        // Test both LSP server and manual implementations
        for use_manual in [false, true] {
            let mock_request = Json(GetCallHierarchyRequest {
                identifier_position: FilePosition {
                    path: String::from("calculator.py"),
                    position: Position {
                        line: 8,
                        character: 12,  // position of Calculator.add method
                    },
                },
                use_manual_hierarchy: use_manual,
            });

            let response = get_call_hierarchy(state.clone(), mock_request).await;

            assert_eq!(response.status(), StatusCode::OK, 
                "Failed with use_manual={}", use_manual);
            assert_eq!(
                response.headers().get("content-type").unwrap(),
                "application/json"
            );

            // Check the body
            let body = response.into_body();
            let bytes = actix_web::body::to_bytes(body).await.unwrap();
            let hierarchy_response: CallHierarchyResponse = serde_json::from_slice(&bytes).unwrap();

            // Basic validation - actual values will depend on the test files
            assert!(!hierarchy_response.items.is_empty(), 
                "Should have at least one item (use_manual={})", use_manual);
            assert!(
                hierarchy_response.items[0].item.name.contains("add"),
                "First item should be Calculator.add method (use_manual={})", use_manual
            );
            
            // Validate structure of first item
            let first_item = &hierarchy_response.items[0];
            assert!(
                first_item.incoming_calls.len() + first_item.outgoing_calls.len() > 0,
                "Should have some calls (use_manual={})", use_manual
            );
            
            // Validate call reference structure
            if let Some(call) = first_item.incoming_calls.first().or(first_item.outgoing_calls.first()) {
                assert!(!call.from.path.is_empty(), 
                    "Call reference should have a path (use_manual={})", use_manual);
                assert!(!call.from.name.is_empty(), 
                    "Call reference should have a name (use_manual={})", use_manual);
                assert!(!call.ranges.is_empty(), 
                    "Call reference should have ranges (use_manual={})", use_manual);
            }
        }
        
        Ok(())
    }

    #[tokio::test]
    async fn test_typescript_call_hierarchy() -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup(&typescript_sample_path(), false).await?;
        let state = initialize_app_state().await?;

        let mock_request = Json(GetCallHierarchyRequest {
            identifier_position: FilePosition {
                path: String::from("src/user.ts"),
                position: Position {
                    line: 5,
                    character: 16,  // position of UserService class method
                },
            },
            use_manual_hierarchy: true,  // Test our manual implementation
        });

        sleep(Duration::from_secs(2)).await;

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
        assert!(!hierarchy_response.items.is_empty(), "Should have at least one item");
        assert!(
            hierarchy_response.items[0].item.name.contains("UserService"),
            "First item should be UserService class"
        );
        
        // Validate structure of first item
        let first_item = &hierarchy_response.items[0];
        assert!(
            first_item.incoming_calls.len() + first_item.outgoing_calls.len() > 0,
            "Should have some calls"
        );
        
        // Validate call reference structure
        if let Some(call) = first_item.incoming_calls.first().or(first_item.outgoing_calls.first()) {
            assert!(!call.from.path.is_empty(), "Call reference should have a path");
            assert!(!call.from.name.is_empty(), "Call reference should have a name");
            assert!(!call.ranges.is_empty(), "Call reference should have ranges");
        }
        
        Ok(())
    }
}