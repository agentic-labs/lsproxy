use crate::api_types::{ErrorResponse, FileRange};
use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use log::{error, info};
use lsp_types::{Position as LspPosition, Range};
use serde::Serialize;
use utoipa::ToSchema;

use crate::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct ReadSourceCodeResponse {
    pub source_code: String,
}

/// Read source code from a file in the workspace
///
/// Returns the contents of the specified file.
#[utoipa::path(
    post,
    path = "/workspace/read-source-code",
    tag = "workspace",
    request_body = FileRange,
    responses(
        (status = 200, description = "Source code retrieved successfully", body = ReadSourceCodeResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn read_source_code(data: Data<AppState>, info: Json<FileRange>) -> HttpResponse {
    info!("Reading source code from file: {}", info.path);

    let manager = data
        .manager
        .lock()
        .map_err(|e| {
            error!("Failed to lock manager: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to lock manager: {}", e),
            })
        })
        .unwrap();

    let lsp_range = Some(Range::new(
        LspPosition {
            line: info.start.line,
            character: info.start.character,
        },
        LspPosition {
            line: info.end.line,
            character: info.end.character,
        },
    ));

    match manager.read_source_code(&info.path, lsp_range).await {
        Ok(source_code) => HttpResponse::Ok().json(ReadSourceCodeResponse { source_code }),
        Err(e) => {
            error!("Failed to read source code: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to read source code: {}", e),
            })
        }
    }
}
