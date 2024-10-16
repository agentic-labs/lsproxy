use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer};
use clap::Parser;
use env_logger::Env;
use log::{debug, error, info};
use lsp_types::Position;
use serde::Deserialize;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use utoipa::{IntoParams, OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

mod api_types;
mod lsp;
mod utils;

use crate::api_types::{
    DefinitionResponse, FilePosition, ReferenceResponse, SupportedLanguages, Symbol,
    SymbolResponse, MOUNT_DIR,
};
use crate::lsp::manager::LspManager;

#[derive(OpenApi)]
#[openapi(
    paths(
        file_symbols,
        workspace_symbols,
        get_definition,
        get_references,
    ),
    components(
        schemas(FileSymbolsRequest, WorkspaceSymbolsRequest, GetDefinitionRequest, GetReferencesRequest, SupportedLanguages, DefinitionResponse, ReferenceResponse, SymbolResponse, FilePosition, Symbol)
    ),
    tags(
        (name = "lsproxy-api", description = "LSP Proxy API")
    )
)]
struct ApiDoc;

#[derive(Deserialize, ToSchema, IntoParams)]
struct GetDefinitionRequest {
    file_path: String,
    line: u32,
    character: u32,
}

#[derive(Deserialize, ToSchema, IntoParams)]
struct GetReferencesRequest {
    file_path: String,
    line: u32,
    character: u32,
    include_declaration: Option<bool>,
}

#[derive(Deserialize, ToSchema, IntoParams)]
struct FileSymbolsRequest {
    file_path: String,
}

#[derive(Deserialize, ToSchema, IntoParams)]
struct WorkspaceSymbolsRequest {
    query: String,
}

struct AppState {
    lsp_manager: Arc<Mutex<LspManager>>,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Write OpenAPI spec to file (openapi.json)
    #[arg(short, long)]
    write_openapi: bool,
}

#[utoipa::path(
    get,
    path = "/definition",
    params(GetDefinitionRequest),
    responses(
        (status = 200, description = "Definition retrieved successfully", body = DefinitionResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_definition(
    data: web::Data<AppState>,
    info: web::Query<GetDefinitionRequest>,
) -> HttpResponse {
    info!(
        "Received get_definition request for file: {}, line: {}, character: {}",
        info.file_path, info.line, info.character
    );

    let full_path = Path::new(&MOUNT_DIR).join(&info.file_path);
    let full_path_str = full_path.to_str().unwrap_or_default();

    match data.lsp_manager.lock() {
        Ok(lsp_manager) => {
            match lsp_manager
                .get_definition(
                    full_path_str,
                    Position {
                        line: info.line,
                        character: info.character,
                    },
                )
                .await
            {
                Ok(definitions) => HttpResponse::Ok().json(definitions),
                Err(e) => {
                    error!("Failed to get definition: {}", e);
                    HttpResponse::InternalServerError().body(e.to_string())
                }
            }
        }
        Err(e) => {
            error!("Failed to lock lsp_manager: {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[utoipa::path(
    get,
    path = "/file-symbols",
    params(FileSymbolsRequest),
    responses(
        (status = 200, description = "Symbols retrieved successfully", body = SymbolResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
async fn file_symbols(
    data: web::Data<AppState>,
    info: web::Query<FileSymbolsRequest>,
) -> HttpResponse {
    info!("Received get_symbols request for file: {}", info.file_path);

    let full_path = Path::new(&MOUNT_DIR).join(&info.file_path);
    let full_path_str = full_path.to_str().unwrap_or("");
    debug!("Full path: {}", full_path_str);

    let result = {
        let lsp_manager = data.lsp_manager.lock().unwrap();
        lsp_manager.file_symbols(full_path_str).await
    };

    match result {
        Ok(symbols) => HttpResponse::Ok().json(symbols),
        Err(e) => {
            error!("Failed to get symbols: {}", e);
            HttpResponse::InternalServerError().body(format!("Failed to get symbols: {}", e))
        }
    }
}

#[utoipa::path(
    get,
    path = "/workspace-symbols",
    params(WorkspaceSymbolsRequest),
    responses(
        (status = 200, description = "Workspace symbols retrieved successfully", body = SymbolResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
async fn workspace_symbols(
    data: web::Data<AppState>,
    info: web::Query<WorkspaceSymbolsRequest>,
) -> HttpResponse {
    info!(
        "Received workspace_symbols request for query: {}",
        info.query
    );

    let result = {
        let lsp_manager = data.lsp_manager.lock().unwrap();
        lsp_manager.workspace_symbols(&info.query).await
    };

    match result {
        Ok(symbols) => HttpResponse::Ok().json(symbols),
        Err(e) => {
            error!("Failed to get workspace symbols: {}", e);
            HttpResponse::InternalServerError()
                .body(format!("Failed to get workspace symbols: {}", e))
        }
    }
}

#[utoipa::path(
    get,
    path = "/references",
    params(GetReferencesRequest),
    responses(
        (status = 200, description = "References retrieved successfully", body = ReferenceResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_references(
    data: web::Data<AppState>,
    info: web::Query<GetReferencesRequest>,
) -> HttpResponse {
    info!(
        "Received get_references request for file: {}, line: {}, character: {}",
        info.file_path, info.line, info.character
    );
    let full_path = Path::new(&MOUNT_DIR).join(&info.file_path);
    let full_path_str = full_path.to_str().unwrap_or("");
    let lsp_manager = data.lsp_manager.lock().unwrap();
    let result = lsp_manager
        .get_references(
            full_path_str,
            Position {
                line: info.line,
                character: info.character,
            },
            info.include_declaration.unwrap_or_else(|| {
                error!("include_declaration not provided, defaulting to true");
                true
            }),
        )
        .await;
    match result {
        Ok(references) => HttpResponse::Ok().json(references),
        Err(e) => {
            error!("Failed to get references: {}", e);
            HttpResponse::InternalServerError().body(format!("Failed to get references: {}", e))
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting...");
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("Server panicked: {:?}", panic_info);
    }));

    env_logger::init_from_env(Env::default().default_filter_or("debug"));
    info!("Logger initialized");

    let cli = Cli::parse();

    let lsp_manager = Arc::new(Mutex::new(LspManager::new()));
    lsp_manager
        .lock()
        .unwrap()
        .start_langservers(MOUNT_DIR)
        .await
        .unwrap();
    let app_state = web::Data::new(AppState { lsp_manager });

    let openapi = ApiDoc::openapi();

    if cli.write_openapi {
        let file_path = PathBuf::from("openapi.json");
        let openapi_json = serde_json::to_string_pretty(&openapi).unwrap();
        let mut file = File::create(&file_path).unwrap();
        file.write_all(openapi_json.as_bytes()).unwrap();
        println!("OpenAPI spec written to: {}", file_path.display());
        return Ok(());
    }

    let server = HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header(),
            )
            .app_data(app_state.clone())
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-docs/openapi.json", openapi.clone()),
            )
            .service(web::resource("/file-symbols").route(web::get().to(file_symbols)))
            .service(web::resource("/workspace-symbols").route(web::get().to(workspace_symbols)))
            .service(web::resource("/definition").route(web::get().to(get_definition)))
            .service(web::resource("/references").route(web::get().to(get_references)))
    })
    .bind("0.0.0.0:8080")?;

    info!("Starting server...");
    server.run().await
}
