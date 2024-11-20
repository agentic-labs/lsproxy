use actix_web::web::{Data, Query};
use actix_web::HttpResponse;
use log::{error, info};

use crate::api_types::{ErrorResponse, FilePosition, FileSymbolsRequest};
use crate::AppState;

#[utoipa::path(
    get,
    path = "/symbol/file-subgraph",
    tag = "symbol",
    params(FileSymbolsRequest),
    responses(
        (status = 200, description = "Symbols retrieved successfully", body = Vec<FilePosition>),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn file_subgraph(data: Data<AppState>, info: Query<FileSymbolsRequest>) -> HttpResponse {
    info!(
        "Received references to imports in file request for file: {}",
        info.file_path
    );
    let manager = match data.manager.lock() {
        Ok(guard) => guard,
        Err(e) => {
            error!("Failed to acquire lock on LSP manager: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Internal server error".to_string(),
            });
        }
    };
    match manager.file_symbol_subgraph(&info.file_path).await {
        Ok(subgraph) => HttpResponse::Ok().json(subgraph),
        Err(e) => HttpResponse::BadRequest().json(ErrorResponse {
            error: format!("Couldn't get file subgraph: {}", e),
        }),
    }
}
