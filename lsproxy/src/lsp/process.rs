use std::error::Error;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::Mutex;

#[async_trait::async_trait]
pub trait Process: Send + Sync {
    async fn send(&mut self, data: &str) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn receive(&self) -> Result<String, Box<dyn Error + Send + Sync>>;
}

#[derive(Clone)]
pub struct ProcessHandler {
    pub stdin: Arc<Mutex<ChildStdin>>,
    pub stdout: Arc<Mutex<BufReader<ChildStdout>>>,
}

impl ProcessHandler {
    pub async fn new(mut child: Child) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let stdin = child.stdin.take().ok_or("Failed to open stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to open stdout")?;
        Ok(Self {
            stdin: Arc::new(Mutex::new(stdin)),
            stdout: Arc::new(Mutex::new(BufReader::new(stdout))),
        })
    }
}

#[async_trait::async_trait]
impl Process for ProcessHandler {
    async fn send(&mut self, data: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut stdin = self.stdin.lock().await;
        stdin.write_all(data.as_bytes()).await?;
        stdin.flush().await?;
        Ok(())
    }

    async fn receive(&self) -> Result<String, Box<dyn Error + Send + Sync>> {
        let mut content_length: Option<usize> = None;
        let mut buffer = Vec::new();

        loop {
            let mut stdout = self.stdout.lock().await;
            let n = stdout.read_until(b'\n', &mut buffer).await?;
            if n == 0 {
                continue;
            }

            let line = String::from_utf8_lossy(&buffer[buffer.len() - n..]);
            if line.trim().is_empty() && content_length.is_some() {
                let length =
                    content_length.ok_or("Missing Content-Length header in LSP message")?;
                let mut content = vec![0; length];
                stdout.read_exact(&mut content).await?;
                return Ok(String::from_utf8(content)?);
            } else if line.starts_with("Content-Length: ") {
                content_length = Some(line.trim_start_matches("Content-Length: ").trim().parse()?);
            }
            buffer.clear();
        }
    }
}
