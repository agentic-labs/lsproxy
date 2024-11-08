use tokio::process::Command;

use super::types::AstGrepMatch;

pub struct AstGrepClient {
    pub root_path: String,
    pub config_path: String,
}

impl AstGrepClient {
    pub async fn get_file_matches(
        &self,
        file_name: &str,
    ) -> Result<Vec<AstGrepMatch>, Box<dyn std::error::Error>> {
        let command_result = Command::new("sg")
            .arg("scan")
            .arg("--config")
            .arg(&self.config_path)
            .arg("--json")
            .arg(file_name)
            .output()
            .await?;

        if !command_result.status.success() {
            let error = String::from_utf8_lossy(&command_result.stderr);
            return Err(format!("sg command failed: {}", error).into());
        }

        let output = String::from_utf8(command_result.stdout)?;

        let mut symbols: Vec<AstGrepMatch> = serde_json::from_str(&output)?;
        symbols.sort_by_key(|s| s.range.start.line);
        Ok(symbols)
    }
}
