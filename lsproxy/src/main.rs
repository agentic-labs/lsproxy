use clap::Parser;
use env_logger::Env;
use log::info;
use lsproxy::{initialize_app_state_with_mount_dir, run_server_with_host, write_openapi_to_file};
use std::path::PathBuf;

/// Command line interface for LSProxy server
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Write OpenAPI specification to openapi.json file
    #[arg(short, long)]
    write_openapi: bool,

    /// Host address to bind the server to
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// Override the default mount directory path where your workspace files are located
    #[arg(long)]
    mount_dir: Option<String>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting...");

    // Set up panic handler for better error reporting
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("Server panicked: {:?}", panic_info);
    }));

    // Initialize logging with info level as default
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    // Parse command line arguments
    let cli = Cli::parse();

    // Handle OpenAPI spec generation if requested
    if cli.write_openapi {
        if let Err(e) = write_openapi_to_file(&PathBuf::from("openapi.json")) {
            eprintln!("Error: Failed to write the openapi.json to a file. Please see error for more details.");
            return Err(e);
        }
        return Ok(());
    }

    // Initialize application state with optional mount directory override
    let app_state = initialize_app_state_with_mount_dir(cli.mount_dir.as_deref())
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    // Run the server with specified host
    run_server_with_host(app_state, &cli.host).await
}
