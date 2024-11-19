use std::{error::Error, os::unix::fs::PermissionsExt, path::Path, process::Stdio};

use async_trait::async_trait;
use log::debug;
use notify_debouncer_mini::DebouncedEvent;
use tokio::{process::Command, sync::broadcast::Receiver};

use crate::{
    lsp::{ExpectedMessageKey, JsonRpcHandler, LspClient, PendingRequests, ProcessHandler},
    utils::workspace_documents::{
        WorkspaceDocumentsHandler, DEFAULT_EXCLUDE_PATTERNS, JAVA_FILE_PATTERNS, JAVA_ROOT_FILES,
    },
};

pub struct JdtlsClient {
    process: ProcessHandler,
    json_rpc: JsonRpcHandler,
    workspace_documents: WorkspaceDocumentsHandler,
    pending_requests: PendingRequests,
}

#[async_trait]
impl LspClient for JdtlsClient {
    fn get_process(&mut self) -> &mut ProcessHandler {
        &mut self.process
    }

    fn get_json_rpc(&mut self) -> &mut JsonRpcHandler {
        &mut self.json_rpc
    }

    fn get_root_files(&mut self) -> Vec<String> {
        JAVA_ROOT_FILES.iter().map(|&s| s.to_string()).collect()
    }

    fn get_workspace_documents(&mut self) -> &mut WorkspaceDocumentsHandler {
        &mut self.workspace_documents
    }

    fn get_pending_requests(&mut self) -> &mut PendingRequests {
        &mut self.pending_requests
    }

    async fn wait_for_ready(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut notification_rx = self
            .get_pending_requests()
            .add_notification(ExpectedMessageKey {
                method: "language/status".to_string(),
                message: "ServiceReady".to_string(),
            })
            .await?;
        
        debug!("Java: waiting for service ready notification. This may take a minute...");
        
        match tokio::time::timeout(
            tokio::time::Duration::from_secs(180),
            notification_rx.recv()
        ).await {
            Ok(Ok(_)) => {
                debug!("Java LSP is ready.");
                Ok(())
            },
            Ok(Err(e)) => {
                self.get_pending_requests()
                    .remove_notification(ExpectedMessageKey {
                        method: "language/status".to_string(),
                        message: "ServiceReady".to_string(),
                    })
                    .await;
                Err(format!("Channel closed while waiting for Java LSP: {}", e).into())
            },
            Err(_) => {
                self.get_pending_requests()
                    .remove_notification(ExpectedMessageKey {
                        method: "language/status".to_string(),
                        message: "ServiceReady".to_string(),
                    })
                    .await;
                Err("Timeout waiting for Java LSP to be ready (180s elapsed)".into())
            }
        }
    }
}

impl JdtlsClient {
    pub async fn new(
        root_path: &str,
        watch_events_rx: Receiver<DebouncedEvent>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let workspace_dir = Path::new("/usr/src/app/jdtls_workspace");
        tokio::fs::create_dir_all(&workspace_dir).await?;
        tokio::fs::set_permissions(&workspace_dir, PermissionsExt::from_mode(0o777)).await?;
        let process = Command::new("java")
            .arg("-Declipse.application=org.eclipse.jdt.ls.core.id1")
            .arg("-Dosgi.bundles.defaultStartLevel=4")
            .arg("-Declipse.product=org.eclipse.jdt.ls.core.product")
            .arg("-Dlog.protocol=true")
            .arg("-Dlog.level=ALL")
            .arg("-Xmx1g")
            .arg("--add-modules=ALL-SYSTEM")
            .arg("--add-opens")
            .arg("java.base/java.util=ALL-UNNAMED")
            .arg("--add-opens")
            .arg("java.base/java.lang=ALL-UNNAMED")
            .arg("-jar")
            .arg("/opt/jdtls/plugins/org.eclipse.equinox.launcher_1.6.900.v20240613-2009.jar")
            .arg("-configuration")
            .arg("/opt/jdtls/config_linux")
            .arg("-data")
            .arg(workspace_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| {
                Box::<dyn std::error::Error + Send + Sync>::from(format!(
                    "Failed to spawn Java process: {}",
                    e
                ))
            })?;

        let process_handler = ProcessHandler::new(process).await.map_err(|e| {
            Box::<dyn std::error::Error + Send + Sync>::from(format!(
                "Failed to create ProcessHandler: {}",
                e
            ))
        })?;

        let workspace_documents = WorkspaceDocumentsHandler::new(
            Path::new(root_path),
            JAVA_FILE_PATTERNS.iter().map(|&s| s.to_string()).collect(),
            DEFAULT_EXCLUDE_PATTERNS
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            watch_events_rx,
        );

        let json_rpc_handler = JsonRpcHandler::new();

        Ok(Self {
            process: process_handler,
            json_rpc: json_rpc_handler,
            workspace_documents,
            pending_requests: PendingRequests::new(),
        })
    }
}
