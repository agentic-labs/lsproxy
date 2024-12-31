use crate::api_types::{HealthResponse, SupportedLanguages};
use crate::AppState;
use actix_web::web::Data;
use actix_web::HttpResponse;
use std::collections::HashMap;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Get health status of the LSP proxy service
///
/// Returns the service status, version and language server availability
#[utoipa::path(
    get,
    path = "/system/health",
    tag = "system",
    responses(
        (status = 200, description = "Health check successful", body = HealthResponse),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn health_check(data: Data<AppState>) -> HttpResponse {
    let manager = match data.manager.lock() {
        Ok(manager) => manager,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(format!("Failed to lock manager: {}", e))
        }
    };

    let mut languages = HashMap::new();
    for lang in [
        SupportedLanguages::Python,
        SupportedLanguages::TypeScriptJavaScript,
        SupportedLanguages::Rust,
        SupportedLanguages::CPP,
        SupportedLanguages::Java,
        SupportedLanguages::Golang,
        SupportedLanguages::PHP,
    ] {
        languages.insert(lang, manager.get_client(lang).is_some());
    }

    HttpResponse::Ok().json(HealthResponse {
        status: "ok".to_string(),
        version: VERSION.to_string(),
        languages,
    })
}
