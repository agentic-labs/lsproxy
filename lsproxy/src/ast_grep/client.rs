use std::io::{Error, ErrorKind};
use tokio::process::Command;

const SYMBOL_CONFIG_PATH: &str = "/usr/src/ast_grep/symbol/config.yml";
const IDENTIFIER_CONFIG_PATH: &str = "/usr/src/ast_grep/identifier/config.yml";
const REFERENCE_CONFIG_PATH: &str = "/usr/src/ast_grep/reference/config.yml";

fn get_symbol_config_path() -> String {
    std::env::var("AST_GREP_SYMBOL_CONFIG").unwrap_or_else(|_| SYMBOL_CONFIG_PATH.to_string())
}

fn get_identifier_config_path() -> String {
    std::env::var("AST_GREP_IDENTIFIER_CONFIG").unwrap_or_else(|_| IDENTIFIER_CONFIG_PATH.to_string())
}

fn get_reference_config_path() -> String {
    std::env::var("AST_GREP_REFERENCE_CONFIG").unwrap_or_else(|_| REFERENCE_CONFIG_PATH.to_string())
}

use super::types::AstGrepMatch;

pub struct AstGrepClient;

impl AstGrepClient {
    pub async fn get_symbol_match_from_position(
        &self,
        file_name: &str,
        identifier_position: &lsp_types::Position,
    ) -> Result<AstGrepMatch, Box<dyn std::error::Error>> {
        // Get all symbols in the file
        let file_symbols = self.scan_file(&get_symbol_config_path(), file_name).await?;

        // Find the symbol that matches our identifier position
        let symbol_result = file_symbols.into_iter().find(|ast_symbol_match| {
            ast_symbol_match.meta_variables.single.name.range.start.line == identifier_position.line
                && ast_symbol_match
                    .meta_variables
                    .single
                    .name
                    .range
                    .start
                    .column
                    == identifier_position.character
        });
        match symbol_result {
            Some(matched_symbol) => Ok(matched_symbol),
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
        self.scan_file(&get_symbol_config_path(), file_name).await
    }

    pub async fn get_file_identifiers(
        &self,
        file_name: &str,
    ) -> Result<Vec<AstGrepMatch>, Box<dyn std::error::Error>> {
        self.scan_file(&get_identifier_config_path(), file_name).await
    }

    pub async fn get_symbol_and_references(
        &self,
        file_name: &str,
        position: &lsp_types::Position,
        full_scan: bool,
    ) -> Result<(AstGrepMatch, Vec<AstGrepMatch>), Box<dyn std::error::Error>> {
        let symbol_match = self
            .get_symbol_match_from_position(file_name, position)
            .await?;
        let references = self
            .get_references_contained_in_symbol_match(file_name, &symbol_match, full_scan)
            .await?;
        Ok((symbol_match, references))
    }

    pub async fn get_references_contained_in_symbol_match(
        &self,
        file_name: &str,
        symbol_match: &AstGrepMatch,
        full_scan: bool,
    ) -> Result<Vec<AstGrepMatch>, Box<dyn std::error::Error>> {
        // Get all references
        let matches = self.scan_file(&get_reference_config_path(), file_name).await?;

        // Filter matches to those within the symbol's range
        // And if not full_scan, exclude matches with rule_id "non-function"
        let contained_references = matches
            .into_iter()
            .filter(|m| {
                let contained = symbol_match.contains(m);
                let all_ref = m.rule_id == "all-references";

                // If we're doing a full scan, we want to use the more permissive "all-references"
                // rule, whereas if we're not doing a full scan, we just want to use the targeted
                // rules
                contained && ((full_scan && all_ref) || (!full_scan && !all_ref))
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
        symbols.sort_by_key(|s| s.get_identifier_range().start.line);
        Ok(symbols)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_references() -> Result<(), Box<dyn std::error::Error>> {
        let client = AstGrepClient {};

        let path = "/mnt/lsproxy_root/sample_project/python/graph.py";
        let position = lsp_types::Position {
            line: 12,
            character: 6,
        };

        let symbol_match = client
            .get_symbol_match_from_position(path, &position)
            .await?;
        let references = client
            .get_references_contained_in_symbol_match(path, &symbol_match, false)
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
        let client = AstGrepClient {};

        let path = "/mnt/lsproxy_root/sample_project/python/main.py";
        let position = lsp_types::Position {
            line: 14,
            character: 4,
        };

        let symbol_match = client
            .get_symbol_match_from_position(path, &position)
            .await?;
        let references = client
            .get_references_contained_in_symbol_match(path, &symbol_match, false)
            .await
            .unwrap();
        let match_positions: Vec<lsp_types::Position> = references
            .iter()
            .map(|ast_match: &AstGrepMatch| lsp_types::Position {
                line: ast_match.get_identifier_range().start.line,
                character: ast_match.get_identifier_range().start.column,
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
