use std::error::Error;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::{
    io::BufReader,
    process::Child,
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    sync::Mutex,
};
use std::sync::Arc;

#[async_trait::async_trait]
pub trait Process: Send + Sync {
    async fn send(&mut self, data: &str) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn receive(&mut self) -> Result<String, Box<dyn Error + Send + Sync>>;
}

pub struct ProcessHandler {
    pub stdin_tx: UnboundedSender<String>,
    pub stdout_rx: Arc<Mutex<UnboundedReceiver<String>>>,
}

impl ProcessHandler {
    pub async fn new(mut child: Child) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let stdin = child.stdin.take().ok_or("Failed to open stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to open stdout")?;

        let (stdin_tx, mut stdin_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let (stdout_tx, stdout_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

        // Spawn a task to handle stdin writes
        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(message) = stdin_rx.recv().await {
                if stdin.write_all(message.as_bytes()).await.is_err() {
                    break;
                }
            }
        });

        // Spawn a task to handle stdout reads
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut content_length: Option<usize> = None;
            let mut buffer = String::new();

            loop {
                buffer.clear();
                match reader.read_line(&mut buffer).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        let line = buffer.trim();
                        
                        // Parse headers
                        if line.starts_with("Content-Length: ") {
                            if let Ok(len) = line["Content-Length: ".len()..].parse::<usize>() {
                                content_length = Some(len);
                            }
                        } else if line.is_empty() && content_length.is_some() {
                            // Empty line indicates end of headers, read the content
                            let mut content = vec![0; content_length.unwrap()];
                            if let Ok(_) = reader.read_exact(&mut content).await {
                                if let Ok(message) = String::from_utf8(content) {
                                    if stdout_tx.send(message).is_err() {
                                        break;
                                    }
                                }
                            }
                            content_length = None;
                        }
                        // Ignore other headers
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            stdin_tx,
            stdout_rx: Arc::new(Mutex::new(stdout_rx)),
        })
    }
}

#[async_trait::async_trait]
impl Process for ProcessHandler {
    async fn send(&mut self, data: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.stdin_tx.send(data.to_string()).map_err(|e| e.into())
    }
    async fn receive(&mut self) -> Result<String, Box<dyn Error + Send + Sync>> {
        match self.stdout_rx.lock().await.recv().await {
            Some(message) => Ok(message),
            None => Err("Failed to receive data".into()),
        }
    }
}
