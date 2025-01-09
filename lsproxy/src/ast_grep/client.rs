use std::io::{Error, ErrorKind};
use tokio::process::Command;

use super::types::AstGrepMatch;
use crate::api_types::{get_mount_dir, FilePosition, Position, Symbol};

pub struct AstGrepClient {
    pub symbol_config_path: String,
    pub identifier_config_path: String,
    pub reference_config_path: String,
}

impl AstGrepClient {
    pub async fn get_symbol_from_position(
        &self,
        file_name: &str,
        identifier_position: &lsp_types::Position,
    ) -> Result<Symbol, Box<dyn std::error::Error>> {
        // Get all symbols in the file
        let full_path = get_mount_dir().join(&file_name);
        let full_path_str = full_path.to_str().unwrap_or_default();
        let file_symbols = self
            .scan_file(&self.symbol_config_path, full_path_str)
            .await?;

        // Find the symbol that matches our identifier position
        let symbol_result = file_symbols.into_iter().find(|symbol| {
            symbol.range.start.line == identifier_position.line
                && symbol.range.start.column == identifier_position.character
        });
        match symbol_result {
            Some(matched_symbol) => Ok(Symbol::from(matched_symbol)),
            None => Err(Box::new(Error::new(
                ErrorKind::NotFound,
                "No symbol found for position",
            ))),
        }
    }

    pub async fn get_file_symbols(
        &self,
        file_name: &str,
    ) -> Result<Vec<AstGrepMatch>, Box<dyn std::error::Error>> {
        self.scan_file(&self.symbol_config_path, file_name).await
    }

    pub async fn get_file_identifiers(
        &self,
        file_name: &str,
    ) -> Result<Vec<AstGrepMatch>, Box<dyn std::error::Error>> {
        self.scan_file(&self.identifier_config_path, file_name)
            .await
    }

    pub async fn get_references_contained_in_symbol(
        &self,
        file_name: &str,
        symbol: &Symbol,
        full_scan: bool,
    ) -> Result<Vec<AstGrepMatch>, Box<dyn std::error::Error>> {
        let full_path = get_mount_dir().join(&file_name);
        let full_path_str = full_path.to_str().unwrap_or_default();

        // Get all references
        let matches = self
            .scan_file(&self.reference_config_path, full_path_str)
            .await?;

        // Filter matches to those within the symbol's range
        // And if not full_scan, exclude matches with rule_id "final-identifier"
        let contained_references = matches
            .into_iter()
            .filter(|m| {
                let position_matches = symbol.range.contains(FilePosition {
                    path: String::from(file_name),
                    position: Position {
                        line: m.range.start.line as u32,
                        character: m.range.start.column as u32,
                    },
                });

                position_matches && (full_scan || m.rule_id != "final-identifier")
            })
            .collect();

        Ok(contained_references)
    }

    async fn scan_file(
        &self,
        config_path: &str,
        file_name: &str,
    ) -> Result<Vec<AstGrepMatch>, Box<dyn std::error::Error>> {
        let command_result = Command::new("ast-grep")
            .arg("scan")
            .arg("--config")
            .arg(config_path)
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
        symbols = symbols.into_iter().collect();
        symbols.sort_by_key(|s| s.range.start.line);
        Ok(symbols)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_references() -> Result<(), Box<dyn std::error::Error>> {
        let client = AstGrepClient {
            symbol_config_path: String::from("/usr/src/ast_grep/symbol-config.yml"),
            identifier_config_path: String::from("/usr/src/ast_grep/identifier-config.yml"),
            reference_config_path: String::from("/usr/src/ast_grep/reference-config.yml"),
        };

        let path = "/mnt/lsproxy_root/sample_project/python/graph.py";
        let position = lsp_types::Position {
            line: 12,
            character: 6,
        };

        let symbol = client.get_symbol_from_position(path, &position).await?;
        let references = client
            .get_references_contained_in_symbol(path, &symbol, false)
            .await?;
        let match_positions: Vec<lsp_types::Position> =
            references.iter().map(lsp_types::Position::from).collect();
        let expected = vec![
            lsp_types::Position {
                line: 15,
                character: 23,
            },
            lsp_types::Position {
                line: 22,
                character: 5,
            },
            lsp_types::Position {
                line: 35,
                character: 15,
            },
            lsp_types::Position {
                line: 35,
                character: 34,
            },
            lsp_types::Position {
                line: 39,
                character: 28,
            },
            lsp_types::Position {
                line: 40,
                character: 29,
            },
            lsp_types::Position {
                line: 63,
                character: 18,
            },
            lsp_types::Position {
                line: 65,
                character: 15,
            },
            lsp_types::Position {
                line: 67,
                character: 5,
            },
            lsp_types::Position {
                line: 71,
                character: 13,
            },
            lsp_types::Position {
                line: 72,
                character: 13,
            },
            lsp_types::Position {
                line: 73,
                character: 46,
            },
            lsp_types::Position {
                line: 75,
                character: 5,
            },
            lsp_types::Position {
                line: 86,
                character: 20,
            },
            lsp_types::Position {
                line: 87,
                character: 18,
            },
        ];
        assert_eq!(match_positions, expected);
        Ok(())
    }

    #[tokio::test]
    async fn test_contained_references() -> Result<(), Box<dyn std::error::Error>> {
        let client = AstGrepClient {
            symbol_config_path: String::from("/usr/src/ast_grep/symbol-config.yml"),
            identifier_config_path: String::from("/usr/src/ast_grep/identifier-config.yml"),
            reference_config_path: String::from("/usr/src/ast_grep/reference-config.yml"),
        };

        let path = "/mnt/lsproxy_root/sample_project/python/main.py";
        let position = lsp_types::Position {
            line: 14,
            character: 4,
        };

        let symbol = client.get_symbol_from_position(path, &position).await?;
        let references = client
            .get_references_contained_in_symbol(path, &symbol, false)
            .await
            .unwrap();
        let match_positions: Vec<lsp_types::Position> = references
            .iter()
            .map(|ast_match: &AstGrepMatch| lsp_types::Position {
                line: ast_match.range.start.line as u32,
                character: ast_match.range.start.column as u32,
            })
            .collect();
        let expected = vec![
            lsp_types::Position {
                line: 15,
                character: 12,
            },
            lsp_types::Position {
                line: 16,
                character: 19,
            },
            lsp_types::Position {
                line: 17,
                character: 4,
            },
            lsp_types::Position {
                line: 18,
                character: 4,
            },
            lsp_types::Position {
                line: 19,
                character: 4,
            },
        ];
        assert_eq!(match_positions, expected);
        Ok(())
    }
}
