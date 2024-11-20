use tokio::process::Command;

use super::types::{AstGrepPatternMatch, AstGrepRuleMatch};

pub struct AstGrepClient {
    pub config_path: String,
}

impl AstGrepClient {
    async fn get_matches(
        &self,
        file_name: &str,
    ) -> Result<Vec<AstGrepRuleMatch>, Box<dyn std::error::Error>> {
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

        let mut matches: Vec<AstGrepRuleMatch> = serde_json::from_str(&output)?;
        matches.sort_by_key(|s| s.range.start.line);
        Ok(matches)
    }

    async fn search(
        &self,
        file_name: &str,
        pattern: &str,
    ) -> Result<Vec<AstGrepPatternMatch>, Box<dyn std::error::Error>> {
        let command_result = Command::new("sg")
            .arg("run")
            .arg(file_name)
            .arg("--pattern")
            .arg(pattern)
            .arg("--json")
            .output()
            .await?;

        let output = String::from_utf8(command_result.stdout)?;
        let matches: Vec<AstGrepPatternMatch> = serde_json::from_str(&output)?;
        Ok(matches)
    }

    pub async fn get_file_symbols(
        &self,
        file_name: &str,
    ) -> Result<Vec<AstGrepRuleMatch>, Box<dyn std::error::Error>> {
        let mut matches = self.get_matches(file_name).await?;
        matches.retain(|s| s.rule_id != "import");
        Ok(matches)
    }

    pub async fn get_file_imports(
        &self,
        file_name: &str,
    ) -> Result<Vec<AstGrepRuleMatch>, Box<dyn std::error::Error>> {
        let mut matches = self.get_matches(file_name).await?;
        matches.retain(|s| s.rule_id == "import");
        Ok(matches)
    }

    pub async fn get_references_to_imports(
        &self,
        import_matches: &Vec<AstGrepRuleMatch>,
    ) -> Result<Vec<AstGrepPatternMatch>, Box<dyn std::error::Error>> {
        if import_matches.is_empty() {
            return Ok(vec![]);
        }
        let file = &import_matches[0].file;
        assert!(import_matches.iter().all(|m| m.file == *file));
        let mut all_matches = vec![];
        let import_positions = import_matches
            .iter()
            .map(|s| &s.range.byte_offset)
            .collect::<Vec<_>>();
        let import_names: Vec<_> = import_matches
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .into_iter()
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        for import_name in import_names {
            let mut matches = self.search(file, import_name).await?;
            matches.retain(|s| !import_positions.contains(&&s.range.byte_offset));
            all_matches.extend(matches);
        }
        all_matches.sort_by_key(|s| (s.range.start.line, s.range.start.column));
        Ok(all_matches)
    }
}
