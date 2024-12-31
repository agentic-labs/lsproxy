use actix_web::web::Data;
use actix_web::HttpResponse;
use log::error;

use crate::api_types::ErrorResponse;
use crate::lsp::manager::LspManagerError;
use crate::AppState;

/// Get a list of all files in the workspace
///
/// Returns an array of file paths for all files in the current workspace.
///
/// This is a convenience endpoint that does not use the underlying Language Servers directly, but it does apply the same filtering.
#[utoipa::path(
    get,
    path = "/workspace/list-files",
    tag = "workspace",
    responses(
        (status = 200, description = "Workspace files retrieved successfully", body = Vec<String>),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_files(data: Data<AppState>) -> HttpResponse {
    let manager = match data.manager.lock() {
        Ok(guard) => guard,
        Err(e) => {
            error!("Failed to acquire manager lock: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Internal server error: failed to acquire lock".to_string(),
            });
        }
    };
    let files = manager.list_files().await;
    match files {
        Ok(files) => HttpResponse::Ok().json(files),
        Err(e) => {
            error!("Failed to get workspace files: {}", e);
            match e {
                LspManagerError::FileNotFound(path) => {
                    HttpResponse::BadRequest().json(ErrorResponse {
                        error: format!("File not found: {}", path),
                    })
                }
                LspManagerError::LspClientNotFound(lang) => HttpResponse::InternalServerError()
                    .json(ErrorResponse {
                        error: format!("LSP client not found for {:?}", lang),
                    }),
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

#[cfg(test)]
mod test {
    use super::*;

    use actix_web::http::StatusCode;

    use crate::initialize_app_state;
    use crate::test_utils::{python_sample_path, TestContext};

    #[tokio::test]
    async fn test_python_workspace_files() -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup(&python_sample_path(), false).await?;
        let state = initialize_app_state().await?;

        let response = list_files(state).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json"
        );

        // Check the body
        let body = response.into_body();
        let bytes = actix_web::body::to_bytes(body).await.unwrap();
        let mut workspace_files_response: Vec<String> = serde_json::from_slice(&bytes).unwrap();

        let mut expected = vec!["graph.py", "main.py", "search.py", "__init__.py"];

        assert_eq!(expected.sort(), workspace_files_response.sort());
        Ok(())
    }
}
