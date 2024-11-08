use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use log::info;
use lsp_types::{GotoDefinitionResponse, Location, Position as LspPosition, Range};

use crate::api_types::{ErrorResponse, FilePosition, FileRange, Position, Symbol, SymbolResponse};
use crate::lsp::manager::{LspManagerError, Manager};
use crate::utils::file_utils::uri_to_relative_path_string;
use crate::AppState;

async fn find_definition_locations_of_references_in_range(
    manager: &Manager,
    file_path: &str,
    range: Range,
) -> Result<Vec<GotoDefinitionResponse>, LspManagerError> {
    let references_in_range = manager.find_references_in_range(file_path, range).await?;
    let definitions_of_references =
        futures::future::try_join_all(references_in_range.into_iter().map(|m| async move {
            manager
                .find_definition(
                    &m.file,
                    LspPosition {
                        line: m.range.start.line as u32,
                        character: m.range.start.column as u32,
                    },
                )
                .await
        }))
        .await?;
    Ok(definitions_of_references)
}

async fn find_definitions_of_references_in_range(
    manager: &Manager,
    file_path: &str,
    range: Range,
) -> Result<Vec<Symbol>, LspManagerError> {
    let definitions_of_references =
        find_definition_locations_of_references_in_range(manager, file_path, range).await?;
    // filter out files not in project
    let workspace_files = manager.list_files().await?;
    let locations: Vec<Location> = definitions_of_references
        .into_iter()
        .flat_map(|def| match def {
            GotoDefinitionResponse::Scalar(location) => vec![location],
            GotoDefinitionResponse::Array(locations) => locations,
            GotoDefinitionResponse::Link(_) => vec![],
        })
        .collect();
    let identifier_positions: Vec<FilePosition> = locations
        .iter()
        .filter(|loc| workspace_files.contains(&uri_to_relative_path_string(&loc.uri)))
        .map(|loc| FilePosition {
            path: uri_to_relative_path_string(&loc.uri),
            position: Position {
                line: loc.range.start.line as u32,
                character: loc.range.start.character as u32,
            },
        })
        .collect();

    let file_paths: Vec<String> = locations
        .iter()
        .map(|loc| uri_to_relative_path_string(&loc.uri))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let all_symbols_by_file =
        futures::future::try_join_all(file_paths.into_iter().map(|file_path| async move {
            let matches = manager.definitions_in_file_ast_grep(&file_path).await?;
            Ok::<Vec<Symbol>, LspManagerError>(matches.into_iter().map(Symbol::from).collect())
        }))
        .await?;

    let mut result = vec![];
    // push symbols who's identifier position is in the identifier_positions
    for symbol in all_symbols_by_file.into_iter().flatten() {
        if identifier_positions.contains(&symbol.identifier_position) {
            result.push(symbol);
        }
    }

    Ok(result)
}

#[utoipa::path(
    post,
    path = "/symbol/find-referenced-definitions",
    tag = "symbol",
    request_body = FileRange,
    responses(
        (status = 200, description = "References retrieved successfully", body = SymbolResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn find_referenced_definitions(
    data: Data<AppState>,
    info: Json<FileRange>,
) -> HttpResponse {
    info!(
        "Received references request for file: {}, line: {}, character: {}",
        info.path, info.start.line, info.start.character
    );
    let manager = data.manager.lock().unwrap();

    let result =
        find_definitions_of_references_in_range(&manager, &info.path, info.clone().into()).await;
    match result {
        Ok(symbols) => HttpResponse::Ok().json(symbols),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: e.to_string(),
        }),
    }
}
