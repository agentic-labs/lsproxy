use crate::api_types::{ErrorResponse, ReadSourceCodeRequest, ReadSourceCodeResponse};
use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use log::{error, info};
use lsp_types::{Position as LspPosition, Range};

use crate::AppState;

/// Read source code from a file in the workspace
///
/// Returns the contents of the specified file.
#[utoipa::path(
    post,
    path = "/workspace/read-source-code",
    tag = "workspace",
    request_body = ReadSourceCodeRequest,
    responses(
        (status = 200, description = "Source code retrieved successfully", body = ReadSourceCodeResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn read_source_code(data: Data<AppState>, req: Json<ReadSourceCodeRequest>) -> HttpResponse {
    info!("Reading source code from file: {}", req.path);

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

    let lsp_range = req.range.as_ref().map(|file_range| Range::new(
        LspPosition {
            line: file_range.start.line,
            character: file_range.start.character,
        },
        LspPosition {
            line: file_range.end.line,
            character: file_range.end.character,
        },
    ));

    match manager.read_source_code(&req.path, lsp_range).await {
        Ok(source_code) => HttpResponse::Ok().json(ReadSourceCodeResponse { source_code }),
        Err(e) => {
            error!("Failed to read source code: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to read source code: {}", e),
            })
        }
    }
}
