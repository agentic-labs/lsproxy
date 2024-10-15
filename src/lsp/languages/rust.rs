use std::{error::Error, fs, process::Stdio};
use async_trait::async_trait;
use config::{Config, File};
use log::{debug,error};
use lsp_types::{InitializeParams, InitializeResult, Url, WorkspaceFolder};
use serde_json::Value;
use tokio::process::Command;

use crate::lsp::{JsonRpc, JsonRpcHandler, LspClient, Process, ProcessHandler};

pub struct RustClient {
    process: ProcessHandler,
    json_rpc: JsonRpcHandler,
}

#[async_trait]
impl LspClient for RustClient {
    fn get_process(&mut self) -> &mut ProcessHandler {
        &mut self.process
    }

    fn get_json_rpc(&mut self) -> &mut JsonRpcHandler {
        &mut self.json_rpc
    }

    async fn setup_workspace(
        &mut self,
        _root_path: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.send_lsp_request("rust-analyzer/reloadWorkspace", None)
            .await?;
        Ok(())
    }

    async fn find_workspace_folders(
        &mut self,
        root_path: String,
    ) -> Result<Vec<WorkspaceFolder>, Box<dyn Error + Send + Sync>> {
        Ok(vec![WorkspaceFolder {
            uri: Url::from_file_path(root_path.clone()).unwrap(),
            name: root_path,
        }])
    }

    async fn initialize(
        &mut self,
        root_path: String,
    ) -> Result<InitializeResult, Box<dyn Error + Send + Sync>> {
        debug!("Initializing LSP client with root path: {:?}", root_path);

        let s = Config::builder()
        .add_source(File::with_name("/config/config.toml"))
        .build()?;
        //if file is specified, parse from json. we will send them back as json later
        let rust_filename = s.get::<String>("rust.file");
        let mut init_options: Option<Value> = None;
        if rust_filename.is_ok(){
            //turn options from file into init_options
            let file = match fs::File::open(format!("/config/{}", rust_filename.unwrap())) {
                Ok(f) => f,
                Err(e) => return Err(Box::new(e) as Box<dyn Error + Send + Sync>),
            };
            init_options = serde_json::from_reader(file).expect("rust config file json parse error");                    
        }

        let params = InitializeParams {
            capabilities: Default::default(),
            workspace_folders: Some(vec![WorkspaceFolder {
                uri: Url::from_file_path(root_path.clone()).unwrap(),
                name: root_path.clone(),
            }]),
            root_uri: Some(Url::from_file_path(root_path.clone()).unwrap()),
            initialization_options: init_options,
            ..Default::default()
        };
        let request = self
            .get_json_rpc()
            .create_request("initialize", serde_json::to_value(params)?);
        let message = format!("Content-Length: {}\r\n\r\n{}", request.len(), request);
        self.get_process().send(&message).await?;
        let response = self.receive_response().await?.expect("No response");
        if let Some(result) = response.result {
            let init_result: InitializeResult = serde_json::from_value(result)?;
            debug!("Initialization successful: {:?}", init_result);
            self.send_initialized().await?;
            Ok(init_result)
        } else if let Some(error) = response.error {
            error!("Initialization error: {:?}", error);
            Err(Box::new(error) as Box<dyn Error + Send + Sync>)
        } else {
            Err("Unexpected initialize response".into())
        }
    }
}

impl RustClient {
    pub async fn new(root_path: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let process = Command::new("rust-analyzer")
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
