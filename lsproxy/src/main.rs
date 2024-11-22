use clap::Parser;
use env_logger::Env;
use log::info;
use lsproxy::{initialize_app_state, initialize_app_state_with_mount_dir, run_server_with_host, write_openapi_to_file};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Write OpenAPI spec to file (openapi.json)
    #[arg(short, long)]
    write_openapi: bool,

    /// Host address to bind to (default: 0.0.0.0)
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    #[arg(short, long)]
    workspace_folder: String,
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

    if cli.write_openapi {
        if let Err(e) = write_openapi_to_file(&PathBuf::from("openapi.json")) {
            eprintln!("Error: Failed to write the openapi.json to a file. Please see error for more details.");
            return Err(e);
        }
        return Ok(());
    }

    let app_state = match cli.workspace_folder {
        w if w.len() > 0 => initialize_app_state_with_mount_dir(Some(&w))
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?,
        _ => initialize_app_state()
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?,
    };

    run_server_with_host(app_state, &cli.host).await
}
