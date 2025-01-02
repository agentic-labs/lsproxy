use tokio::process::Command;

use super::types::AstGrepMatch;

pub struct AstGrepClient {
    pub config_path: String,
}

impl AstGrepClient {
    pub async fn get_file_symbols(
        &self,
        file_name: &str,
    ) -> Result<Vec<AstGrepMatch>, Box<dyn std::error::Error>> {
        let command_result = Command::new("ast-grep")
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

        let mut symbols: Vec<AstGrepMatch> =
            serde_json::from_str(&output).map_err(|e| format!("Failed to parse JSON: {}", e))?;
        symbols = symbols
            .into_iter()
            .filter(|s| s.rule_id != "all-identifiers")
            .collect();
        symbols.sort_by_key(|s| s.range.start.line);
        Ok(symbols)
    }

    pub async fn get_file_identifiers(
        &self,
        file_name: &str,
    ) -> Result<Vec<AstGrepMatch>, Box<dyn std::error::Error>> {
        let command_result = Command::new("ast-grep")
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
        let mut identifiers: Vec<AstGrepMatch> =
            serde_json::from_str(&output).map_err(|e| format!("Failed to parse JSON: {}", e))?;
        identifiers = identifiers
            .into_iter()
            .filter(|s| s.rule_id == "all-identifiers")
            .collect();

        identifiers.sort_by_key(|s| s.range.start.line);
        Ok(identifiers)
    }
}
