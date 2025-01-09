use actix_web::{
    web::{Data, Json},
    HttpResponse,
};

use crate::{
    api_types::{
        ErrorResponse, FilePosition, FindIdentifierRequest, Identifier, IdentifierResponse,
    },
    handlers::utils::{self, PositionError},
    AppState,
};
use log::{error, info};

/// Finds occurrences of an identifier by name in a file
///
/// Given a file path and identifier name, returns:
/// - Without position: All matching identifiers in the file
/// - With position: The exact identifier with that name at that position, or 3 closest identifiers with that name
///
/// Example finding all occurrences of "user_name":
/// ```
/// let user_name = "John";  // First occurrence
/// println!("{}", user_name); // Second occurrence
/// ```
///
/// When a position is provided, it searches for an exact match at that location.
/// If no exact match exists, returns the 3 identifiers closest to the position
/// based on line and character distance, prioritizing lines.
#[utoipa::path(
    post,
    path = "/symbol/find-identifier",
    tag = "symbol",
    request_body = FindIdentifierRequest,
    responses(
        (status = 200, description = "Identifier retrieved successfully", body = IdentifierResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn find_identifier(
    data: Data<AppState>,
    info: Json<FindIdentifierRequest>,
) -> HttpResponse {
    info!(
        "Received identifier request for file: {}, name: {}, position: {:?}",
        info.path, info.name, info.position
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
    let file_identifiers = match manager.get_file_identifiers(&info.path).await {
        Ok(identifiers) => identifiers,
        Err(e) => {
            error!("Failed to get file identifiers: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to get file identifiers: {}", e),
            });
        }
    };

    // filter identifiers by name
    let name_matched_identifiers: Vec<Identifier> = file_identifiers
        .into_iter()
        .filter(|id| id.name == info.name)
        .collect();

    if name_matched_identifiers.is_empty() {
        return HttpResponse::Ok().json(IdentifierResponse {
            identifiers: vec![],
        });
    }

    if let Some(position) = &info.position {
        match utils::find_identifier_at_position(
            name_matched_identifiers.clone(),
            &FilePosition {
                path: info.path.clone(),
                position: position.clone(),
            },
        )
        .await
        {
            Ok(identifier) => HttpResponse::Ok().json(IdentifierResponse {
                identifiers: vec![identifier],
            }),
            Err(PositionError::IdentifierNotFound { closest }) => {
                // Not an error case, just closest matches
                HttpResponse::Ok().json(IdentifierResponse {
                    identifiers: closest,
                })
            }
        }
    } else {
        HttpResponse::Ok().json(IdentifierResponse {
            identifiers: name_matched_identifiers,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::api_types::Position;
    use crate::initialize_app_state;
    use crate::test_utils::{python_sample_path, typescript_sample_path, TestContext};
    use actix_web::http::StatusCode;

    #[tokio::test]
    async fn test_python_find_all_identifiers() -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup(&python_sample_path(), false).await?;
        let state = initialize_app_state().await?;

        // Test finding all occurrences of 'graph' without position
        let mock_request = Json(FindIdentifierRequest {
            path: String::from("main.py"),
            name: String::from("graph"),
            position: None,
        });

        let response = find_identifier(state, mock_request).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body();
        let bytes = actix_web::body::to_bytes(body).await.unwrap();
        let identifier_response: IdentifierResponse = serde_json::from_slice(&bytes).unwrap();

        // Should find at least two occurrences: declaration and usage
        assert!(identifier_response.identifiers.len() >= 2);
        assert!(identifier_response
            .identifiers
            .iter()
            .all(|id| id.name == "graph"));
        Ok(())
    }

    #[tokio::test]
    async fn test_python_find_exact_identifier() -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup(&python_sample_path(), false).await?;
        let state = initialize_app_state().await?;

        // Test finding exact occurrence of 'AStarGraph' at its definition
        let mock_request = Json(FindIdentifierRequest {
            path: String::from("graph.py"),
            name: String::from("AStarGraph"),
            position: Some(Position {
                line: 12,
                character: 6,
            }),
        });

        let response = find_identifier(state, mock_request).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body();
        let bytes = actix_web::body::to_bytes(body).await.unwrap();
        let identifier_response: IdentifierResponse = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(identifier_response.identifiers.len(), 1);
        assert_eq!(identifier_response.identifiers[0].name, "AStarGraph");
        assert_eq!(identifier_response.identifiers[0].range.start.line, 12);
        Ok(())
    }

    #[tokio::test]
    async fn test_typescript_find_all_identifiers() -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup(&typescript_sample_path(), false).await?;
        let state = initialize_app_state().await?;

        // Test finding all occurrences of 'path' in PathfinderDisplay.tsx
        let mock_request = Json(FindIdentifierRequest {
            path: String::from("PathfinderDisplay.tsx"),
            name: String::from("path"),
            position: None,
        });

        let response = find_identifier(state, mock_request).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body();
        let bytes = actix_web::body::to_bytes(body).await.unwrap();
        let identifier_response: IdentifierResponse = serde_json::from_slice(&bytes).unwrap();

        // Should find multiple occurrences in state and JSX
        assert!(identifier_response.identifiers.len() >= 2);
        assert!(identifier_response
            .identifiers
            .iter()
            .all(|id| id.name == "path"));
        Ok(())
    }

    #[tokio::test]
    async fn test_typescript_find_closest_matches() -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup(&typescript_sample_path(), false).await?;
        let state = initialize_app_state().await?;

        // Test finding closest matches for 'maze' near but not exactly at a position
        let mock_request = Json(FindIdentifierRequest {
            path: String::from("PathfinderDisplay.tsx"),
            name: String::from("maze"),
            position: Some(Position {
                line: 25, // Near maze usage but not exact
                character: 10,
            }),
        });

        let response = find_identifier(state, mock_request).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body();
        let bytes = actix_web::body::to_bytes(body).await.unwrap();
        let identifier_response: IdentifierResponse = serde_json::from_slice(&bytes).unwrap();

        // Should return up to 3 closest matches
        assert!(identifier_response.identifiers.len() <= 3);
        assert!(identifier_response
            .identifiers
            .iter()
            .all(|id| id.name == "maze"));
        Ok(())
    }

    #[tokio::test]
    async fn test_invalid_file() -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup(&python_sample_path(), false).await?;
        let state = initialize_app_state().await?;

        let mock_request = Json(FindIdentifierRequest {
            path: String::from("nonexistent.py"),
            name: String::from("identifier"),
            position: None,
        });

        let response = find_identifier(state, mock_request).await;
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = response.into_body();
        let bytes = actix_web::body::to_bytes(body).await.unwrap();
        let error_response: ErrorResponse = serde_json::from_slice(&bytes).unwrap();

        assert!(error_response
            .error
            .contains("Failed to get file identifiers"));
        Ok(())
    }

    #[tokio::test]
    async fn test_no_matches_found() -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup(&python_sample_path(), false).await?;
        let state = initialize_app_state().await?;

        let mock_request = Json(FindIdentifierRequest {
            path: String::from("main.py"),
            name: String::from("nonexistent_identifier"),
            position: None,
        });

        let response = find_identifier(state, mock_request).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body();
        let bytes = actix_web::body::to_bytes(body).await.unwrap();
        let identifier_response: IdentifierResponse = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(identifier_response.identifiers.len(), 0);
        Ok(())
    }
}
