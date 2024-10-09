use actix_web::{web, App, HttpServer, HttpResponse, Responder};
use actix_cors::Cors;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Mutex;
use tempfile::TempDir;
use git2::{Repository, BranchType};
use log::{info, error, debug};
use env_logger::Env;
use std::path::PathBuf;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    paths(
        clone_repo,
        list_repos,
        init_lsp,
        get_function_definition,
        list_lsp_servers,
        get_document_symbols
    ),
    components(
        schemas(CloneRequest, RepoInfo, LspInitRequest, FunctionDefinitionRequest, SymbolRequest)
    ),
    tags(
        (name = "github-clone-api", description = "GitHub Clone API")
    )
)]
struct ApiDoc;

#[derive(Deserialize, utoipa::ToSchema)]
struct CloneRequest {
    id: String,
    github_url: String,
    reference: Option<String>,
}

#[derive(Serialize, Clone, utoipa::ToSchema)]
struct RepoInfo {
    id: String,
    github_url: String,
    branch: Option<String>,
    commit: String,
    temp_dir: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
struct LspInitRequest {
    id: String,
    github_url: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
struct FunctionDefinitionRequest {
    id: String,
    github_url: String,
    file_path: String,
    line: u32,
    character: u32,
}

#[derive(Deserialize, utoipa::ToSchema)]
struct SymbolRequest {
    id: String,
    github_url: String,
    file_path: String,
}

mod lsp_manager;
mod lsp_client;

use lsp_manager::LspManager;

struct AppState {
    clones: Mutex<HashMap<String, HashMap<String, (TempDir, RepoInfo)>>>,
    lsp_manager: Mutex<LspManager>,
}

fn get_branch_and_commit(repo: &Repository, reference: &str) -> Result<(Option<String>, String), git2::Error> {
    let obj = repo.revparse_single(reference)?;
    let commit = obj.peel_to_commit()?;
    let commit_id = commit.id().to_string();

    if let Ok(branch) = repo.find_branch(reference, BranchType::Local) {
        Ok((Some(branch.name()?.unwrap_or("").to_string()), commit_id))
    } else if let Ok(branch) = repo.find_branch(reference, BranchType::Remote) {
        Ok((Some(branch.name()?.unwrap_or("").to_string()), commit_id))
    } else {
        let branches = repo.branches(None)?;
        let branch = branches
            .filter_map(|b| b.ok())
            .find(|(branch, _)| branch.get().target() == Some(commit.id()));
        
        match branch {
            Some((branch, _)) => Ok((branch.name()?.map(String::from), commit_id)),
            None => Ok((None, commit_id)),
        }
    }
}

#[utoipa::path(
    post,
    path = "/clone",
    request_body = CloneRequest,
    responses(
        (status = 200, description = "Repository cloned successfully", body = RepoInfo),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
async fn clone_repo(
    data: web::Data<AppState>,
    info: web::Json<CloneRequest>,
) -> HttpResponse {
    info!("Received clone request for ID: {}, URL: {}", info.id, info.github_url);

    let temp_dir = match TempDir::new() {
        Ok(dir) => dir,
        Err(e) => {
            error!("Failed to create temp directory: {}", e);
            return HttpResponse::InternalServerError().body("Failed to create temp directory");
        }
    };

    let repo = match Repository::clone(&info.github_url, temp_dir.path()) {
        Ok(repo) => repo,
        Err(e) => {
            error!("Failed to clone repository: {}", e);
            return HttpResponse::BadRequest().body("Failed to clone repository");
        }
    };

    let (branch, commit) = match &info.reference {
        Some(ref_name) => {
            debug!("Checking out reference: {}", ref_name);
            match get_branch_and_commit(&repo, ref_name) {
                Ok((branch, commit)) => {
                    if let Err(e) = checkout_reference(&repo, ref_name) {
                        error!("Failed to checkout specified reference: {}", e);
                        return HttpResponse::BadRequest().body("Failed to checkout specified reference");
                    }
                    (branch, commit)
                },
                Err(e) => {
                    error!("Failed to get branch and commit info: {}", e);
                    return HttpResponse::BadRequest().body("Failed to get branch and commit info");
                }
            }
        },
        None => {
            match repo.head() {
                Ok(head) => {
                    match head.peel_to_commit() {
                        Ok(commit_obj) => {
                            let commit = commit_obj.id().to_string();
                            (head.shorthand().map(String::from), commit)
                        },
                        Err(e) => {
                            error!("Failed to get commit: {}", e);
                            return HttpResponse::InternalServerError().body("Failed to get commit");
                        }
                    }
                },
                Err(e) => {
                    error!("Failed to get repository head: {}", e);
                    return HttpResponse::InternalServerError().body("Failed to get repository head");
                }
            }
        }
    };

    let repo_info = RepoInfo {
        id: info.id.clone(),
        github_url: info.github_url.clone(),
        branch,
        commit,
        temp_dir: temp_dir.path().to_string_lossy().into_owned(),
    };

    let mut clones = data.clones.lock().unwrap();
    clones
        .entry(info.id.clone())
        .or_insert_with(HashMap::new)
        .insert(info.github_url.clone(), (temp_dir, repo_info.clone()));

    info!("Repository cloned successfully. ID: {}, URL: {}, Branch: {:?}, Commit: {}", 
          info.id, info.github_url, repo_info.branch, repo_info.commit);
    HttpResponse::Ok().json(repo_info)
}

fn checkout_reference(repo: &Repository, reference: &str) -> Result<(), git2::Error> {
    if let Ok(oid) = repo.revparse_single(reference).and_then(|obj| obj.peel_to_commit()) {
        repo.checkout_tree(&oid.as_object(), None)?;
        repo.set_head_detached(oid.id())?;
    } else {
        let branch = repo.find_branch(reference, BranchType::Remote)?;
        let commit = branch.get().peel_to_commit()?;
        repo.checkout_tree(&commit.as_object(), None)?;
        repo.set_head(branch.get().name().unwrap())?;
    }
    Ok(())
}

#[utoipa::path(
    get,
    path = "/list",
    responses(
        (status = 200, description = "List of cloned repositories", body = Vec<RepoInfo>)
    )
)]
async fn list_repos(data: web::Data<AppState>) -> HttpResponse {
    let clones = data.clones.lock().unwrap();
    let repo_list: Vec<RepoInfo> = clones
        .iter()
        .flat_map(|(_, repos)| {
            repos.values().map(|(_, repo_info)| repo_info.clone())
        })
        .collect();

    HttpResponse::Ok().json(repo_list)
}

#[utoipa::path(
    post,
    path = "/init-lsp",
    request_body = LspInitRequest,
    responses(
        (status = 200, description = "LSP server initialized successfully"),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
async fn init_lsp(
    data: web::Data<AppState>,
    info: web::Json<LspInitRequest>,
) -> HttpResponse {
    info!("Received LSP init request for ID: {}, URL: {}", info.id, info.github_url);

    let clones = data.clones.lock().unwrap();
    let repo_map = match clones.get(&info.id) {
        Some(map) => map,
        None => {
            return HttpResponse::BadRequest().body("No repositories found for the given ID");
        }
    };

    let (_, repo_info) = match repo_map.get(&info.github_url) {
        Some(entry) => entry,
        None => {
            return HttpResponse::BadRequest().body("Repository not found");
        }
    };

    let repo_path = PathBuf::from(&repo_info.temp_dir);
    let mut lsp_manager = data.lsp_manager.lock().unwrap();
    
    match lsp_manager.start_lsp_for_repo(repo_path.clone()).await {
        Ok(_) => {
            info!("LSP server started for repo: {}", repo_info.temp_dir);
            HttpResponse::Ok().body(format!("LSP server started for repo: {}", repo_info.temp_dir))
        },
        Err(e) => {
            HttpResponse::InternalServerError().body(format!("Failed to start LSP server: {}", e))
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));
    info!("Starting server at http://0.0.0.0:8080");

    let app_state = web::Data::new(AppState {
        clones: Mutex::new(HashMap::new()),
        lsp_manager: Mutex::new(LspManager::new()),
    });

    let openapi = ApiDoc::openapi();

    info!("Initializing HTTP server");
    let server = HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();

        App::new()
            .wrap(cors)
            .app_data(app_state.clone())
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", openapi.clone())
            )
            .service(web::resource("/clone").route(web::post().to(clone_repo)))
            .service(web::resource("/list").route(web::get().to(list_repos)))
            .service(web::resource("/init-lsp").route(web::post().to(init_lsp)))
            .service(web::resource("/function-definition").route(web::post().to(get_function_definition)))
            .service(web::resource("/list-lsp").route(web::get().to(list_lsp_servers)))
            .service(web::resource("/document-symbols").route(web::post().to(get_document_symbols)))
    })
    .bind("0.0.0.0:8080")?;

    info!("Server bound to 0.0.0.0:8080");
    info!("Starting server...");
    let result = server.run().await;
    info!("Server stopped");
    result
}

#[utoipa::path(
    post,
    path = "/function-definition",
    request_body = FunctionDefinitionRequest,
    responses(
        (status = 200, description = "Function definition retrieved successfully"),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_function_definition(
    data: web::Data<AppState>,
    info: web::Json<FunctionDefinitionRequest>,
) -> HttpResponse {
    info!("Received function definition request for ID: {}, URL: {}", info.id, info.github_url);

    let clones = data.clones.lock().unwrap();
    let repo_map = match clones.get(&info.id) {
        Some(map) => map,
        None => {
            return HttpResponse::BadRequest().body("No repositories found for the given ID");
        }
    };

    let (_, repo_info) = match repo_map.get(&info.github_url) {
        Some(entry) => entry,
        None => {
            return HttpResponse::BadRequest().body("Repository not found");
        }
    };

    let repo_path = PathBuf::from(&repo_info.temp_dir);
    let mut lsp_manager = data.lsp_manager.lock().unwrap();
    
    let lsp_client = match lsp_manager.get_lsp_for_repo(&repo_path) {
        Some(client) => client,
        None => {
            return HttpResponse::InternalServerError().body("LSP server not found for this repository");
        }
    };

    let params = json!({
        "textDocument": {
            "uri": format!("file://{}/{}", repo_info.temp_dir, info.file_path)
        },
        "position": {
            "line": info.line,
            "character": info.character
        }
    });

    match lsp_client.send_request("textDocument/definition", params).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => HttpResponse::InternalServerError().body(format!("Failed to get function definition: {}", e))
    }
}

#[utoipa::path(
    get,
    path = "/list-lsp",
    responses(
        (status = 200, description = "List of active LSP servers", body = Vec<String>)
    )
)]
async fn list_lsp_servers(data: web::Data<AppState>) -> HttpResponse {
    let lsp_manager = data.lsp_manager.lock().unwrap();
    let active_servers = lsp_manager.list_active_lsp_servers();
    
    let server_list: Vec<String> = active_servers
        .into_iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();

    HttpResponse::Ok().json(server_list)
}

#[utoipa::path(
    post,
    path = "/document-symbols",
    request_body = SymbolRequest,
    responses(
        (status = 200, description = "Document symbols retrieved successfully"),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_document_symbols(
    data: web::Data<AppState>,
    info: web::Json<SymbolRequest>,
) -> HttpResponse {
    info!("Received document symbols request for ID: {}, URL: {}, File: {}", info.id, info.github_url, info.file_path);

    let clones = data.clones.lock().unwrap();
    let repo_map = match clones.get(&info.id) {
        Some(map) => map,
        None => {
            error!("No repositories found for ID: {}", info.id);
            return HttpResponse::BadRequest().body("No repositories found for the given ID");
        }
    };

    let (_, repo_info) = match repo_map.get(&info.github_url) {
        Some(entry) => entry,
        None => {
            error!("Repository not found for URL: {}", info.github_url);
            return HttpResponse::BadRequest().body("Repository not found");
        }
    };

    let repo_path = PathBuf::from(&repo_info.temp_dir);
    let mut lsp_manager = data.lsp_manager.lock().unwrap();
    
    let lsp_client = match lsp_manager.get_lsp_for_repo(&repo_path) {
        Some(client) => client,
        None => {
            error!("LSP server not found for repo path: {:?}", repo_path);
            return HttpResponse::InternalServerError().body("LSP server not found for this repository");
        }
    };

    let file_uri = format!("file://{}/{}", repo_info.temp_dir, info.file_path);
    info!("Requesting symbols for file: {}", file_uri);

    let params = json!({
        "textDocument": {
            "uri": file_uri
        }
    });

    info!("Sending LSP request: textDocument/documentSymbol");
    match lsp_client.send_request("textDocument/documentSymbol", params).await {
        Ok(response) => {
            info!("Received LSP response for document symbols");
            debug!("LSP response: {:?}", response);
            HttpResponse::Ok().json(response)
        },
        Err(e) => {
            error!("Failed to get document symbols: {}", e);
            HttpResponse::InternalServerError().body(format!("Failed to get document symbols: {}", e))
        }
    }
}
