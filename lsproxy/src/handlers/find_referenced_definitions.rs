use std::path::PathBuf;

use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use log::{debug, info};
use lsp_types::{GotoDefinitionResponse, Location, Position as LspPosition, Range};

use crate::api_types::{ErrorResponse, FilePosition, FileRange, Position, Symbol, SymbolResponse};
use crate::lsp::manager::{LspManagerError, Manager};
use crate::utils::file_utils::{
    absolute_path_to_relative_path_string, uri_to_relative_path_string,
};
use crate::AppState;

async fn find_definition_locations_of_references_in_range(
    manager: &Manager,
    file_path: &str,
    range: Range,
) -> Result<Vec<GotoDefinitionResponse>, LspManagerError> {
    let references_in_range = manager.find_references_in_range(file_path, range).await?;
    debug!("references_in_range: {:?}", references_in_range);
    let mut definitions_of_references = Vec::new();
    for m in references_in_range {
        info!("Finding definition for reference at {:?}", m);
        match manager
            .find_definition(
                &absolute_path_to_relative_path_string(&PathBuf::from(&m.file)),
                LspPosition {
                    line: m.range.start.line as u32,
                    character: m.range.start.column as u32,
                },
            )
            .await
        {
            Ok(def) => definitions_of_references.push(def),
            Err(e) => panic!("Failed to find definition: {:?}", e),
        }
    }
    Ok(definitions_of_references)
}

async fn find_definitions_of_references_in_range(
    manager: &Manager,
    file_path: &str,
    range: Range,
) -> Result<Vec<Symbol>, LspManagerError> {
    let definitions_of_references =
        find_definition_locations_of_references_in_range(manager, file_path, range).await?;
    debug!("definitions_of_references: {:?}", definitions_of_references);
    // filter out files not in project
    let workspace_files = manager.list_files().await?;
    let locations: Vec<Location> = definitions_of_references
        .into_iter()
        .flat_map(|def| match def {
            GotoDefinitionResponse::Scalar(location) => vec![location],
            GotoDefinitionResponse::Array(locations) => locations,
            GotoDefinitionResponse::Link(_) => vec![],
            _ => panic!("Unknown GotoDefinitionResponse variant {:?}", def),
        })
        .collect();
    debug!("locations: {:?}", locations);
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
    debug!("identifier_positions: {:?}", identifier_positions);
    let file_paths: Vec<String> = identifier_positions
        .iter()
        .map(|pos| pos.path.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    debug!("file_paths: {:?}", file_paths);
    let all_symbols_by_file =
        futures::future::try_join_all(file_paths.into_iter().map(|file_path| async move {
            let matches = manager.definitions_in_file_ast_grep(&file_path).await?;
            Ok::<Vec<Symbol>, LspManagerError>(matches.into_iter().map(Symbol::from).collect())
        }))
        .await?;
    debug!("all_symbols_by_file: {:?}", all_symbols_by_file);
    let mut result = vec![];
    // push symbols who's identifier position is in the identifier_positions
    for symbol in all_symbols_by_file.into_iter().flatten() {
        if identifier_positions.contains(&symbol.identifier_position) {
            result.push(symbol);
        }
    }
    debug!("result: {:?}", result);
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

#[cfg(test)]
mod test {
    use std::time::Duration;

    use super::*;

    use actix_web::http::StatusCode;
    use tokio::time::sleep;

    use crate::api_types::Position;
    use crate::initialize_app_state;
    use crate::test_utils::{python_sample_path, TestContext};

    #[tokio::test]
    async fn test_python_referenced_definitions() -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup(&python_sample_path(), false).await?;
        let state = initialize_app_state().await?;
        sleep(Duration::from_secs(10)).await;

        let mock_request = Json(FileRange {
            path: String::from("search.py"),
            start: Position {
                line: 11,
                character: 0,
            },
            end: Position {
                line: 48,
                character: 0,
            },
        });

        let response = find_referenced_definitions(state, mock_request).await;

        assert_eq!(response.status(), StatusCode::OK, "{:?}", response.body());
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json"
        );

        let body = response.into_body();
        let bytes = actix_web::body::to_bytes(body).await.unwrap();
        let definition_response: SymbolResponse = serde_json::from_slice(&bytes).unwrap();

        let expected_response: Vec<Symbol> = vec![];

        assert_eq!(expected_response, definition_response);
        Ok(())
    }
}
