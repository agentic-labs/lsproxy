use actix_web::{
    web::{Data, Json},
    HttpResponse,
};
use lsp_types::Position as LspPosition;

use crate::{
    api_types::{ErrorResponse, RenameRequest, RenameResponse},
    AppState,
};

#[utoipa::path(
    post,
    path = "/symbol/rename",
    tag = "symbol",
    request_body = RenameRequest,
    responses(
        (status = 200, description = "Symbol renamed successfully", body = RenameResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn rename(data: Data<AppState>, info: Json<RenameRequest>) -> HttpResponse {
    let manager = data
        .manager
        .lock()
        .map_err(|e| {
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to lock manager: {}", e),
            })
        })
        .unwrap_or_else(|_| {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to acquire manager lock".to_string(),
            });
        });

    let result = manager
        .rename_symbol(
            &info.position.path,
            LspPosition {
                line: info.position.position.line,
                character: info.position.position.character,
            },
            info.new_name.clone(),
        )
        .await;
    match result {
        Ok(edit) => HttpResponse::Ok().json(RenameResponse::from(edit)),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to rename symbol: {}", e),
        }),
    }
}
