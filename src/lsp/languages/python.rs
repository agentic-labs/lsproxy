use std::{collections::HashMap, fs, process::Stdio};

use crate::lsp::{JsonRpcHandler, LspClient, ProcessHandler};
use async_trait::async_trait;
use log::debug;
use std::path::Path;
use tokio::process::Command;

use config::{Config, File};

pub struct PythonClient {
    process: ProcessHandler,
    json_rpc: JsonRpcHandler,
}

#[async_trait]
impl LspClient for PythonClient {
    fn get_process(&mut self) -> &mut ProcessHandler {
        &mut self.process
    }

    fn get_json_rpc(&mut self) -> &mut JsonRpcHandler {
        &mut self.json_rpc
    }
}

impl PythonClient {
    pub async fn new(root_path: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        //if pyrightconfig already exists in the project directory or if the pyproject.toml contains a "tool.pyright" section, skip configuration
        let mut pyright_test = Path::new(&format!("{}/pyrightconfig.json", root_path)).exists();
        if !pyright_test {
            //test pyproject.toml
            match Config::builder()
                .add_source(File::with_name(&format!("{}/pyproject.toml", root_path)))
                .build()
            {
                Ok(s) => {
                    match s.get::<HashMap<String, String>>("tool.pyright") {
                        Ok(_) => pyright_test = true,
                        Err(_) => {
                            debug!("found pyproject.toml but didn't contain a tool.pyright section, initializing using config/pyrightconfig.json if it exists.")
                        }
                    };
                }
                Err(_) => {
                    debug!("Checked for pyproject.toml and it either didn't exist or we couldn't parse it, initializing using config/pyrightconfig.json if it exists")
                }
            };
        }
        if !pyright_test {
            //pyright initialization works best by simply allowing the server to read the proper file on startup.
            //first parse our config.toml file to see if that's necessary
            let s = Config::builder()
                .add_source(File::with_name("/config/config.toml"))
                .build()?;
            //if file is specified, but file_type is either not specified or something other than "pyrightconfig", we assume it's a pyproject.toml file.
            let python_filename = s.get::<String>("python.file");
            if python_filename.is_ok() {
                //copy the file included on run to the proper location.
                let python_filetype = s.get::<String>("python.file_type");
                if python_filetype.is_ok() {
                    if python_filetype.unwrap() != "pyrightconfig" {
                        if let Err(e) = fs::copy(
                            format!("/config/{}", python_filename.unwrap()),
                            format!("{}/pyproject.toml", root_path),
                        ) {
                            eprintln!("Failed to copy pyproject.toml: {:?}", e);
                        }
                    } else {
                        if let Err(e) = fs::copy(
                            format!("/config/{}", python_filename.unwrap()),
                            format!("{}/pyrightconfig.json", root_path),
                        ) {
                            eprintln!("Failed to copy pyrightconfig.json: {:?}", e);
                        }
                    }
                }
            }
        }

        let process = Command::new("pyright-langserver")
            .arg("--stdio")
            .current_dir(root_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let process_handler = ProcessHandler::new(process)
            .await
            .map_err(|e| format!("Failed to create ProcessHandler: {}", e))?;
        let json_rpc_handler = JsonRpcHandler::new();

        Ok(Self {
            process: process_handler,
            json_rpc: json_rpc_handler,
        })
    }
}
