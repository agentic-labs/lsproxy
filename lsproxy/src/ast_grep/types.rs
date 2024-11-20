use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    api_types::{FilePosition, FileRange, Position, Symbol},
    utils::file_utils::absolute_path_to_relative_path_string,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AstGrepRuleMatch {
    pub text: String,
    pub range: AstGrepRange,
    pub file: String,
    pub lines: String,
    pub char_count: CharCount,
    pub language: String,
    pub meta_variables: MetaVariables,
    pub rule_id: String,
    pub labels: Vec<Label>,
}

impl AstGrepRuleMatch {
    pub fn get_source_code(&self) -> Option<String> {
        self.meta_variables
            .multi
            .secondary
            .last()
            .map(|s| s.text.clone())
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AstGrepPatternMatch {
    pub text: String,
    pub range: AstGrepRange,
    pub file: String,
    pub lines: String,
    pub char_count: CharCount,
    pub language: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AstGrepRange {
    pub byte_offset: ByteOffset,
    pub start: AstGrepPosition,
    pub end: AstGrepPosition,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ByteOffset {
    pub start: usize,
    pub end: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AstGrepPosition {
    pub line: usize,
    pub column: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CharCount {
    pub leading: usize,
    pub trailing: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MetaVariables {
    pub multi: MultiVariables,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MultiVariables {
    pub secondary: Vec<Secondary>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Secondary {
    pub text: String,
    pub range: AstGrepRange,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    pub text: String,
    pub range: AstGrepRange,
}

impl From<AstGrepRuleMatch> for Symbol {
    fn from(ast_match: AstGrepRuleMatch) -> Self {
        let path = absolute_path_to_relative_path_string(&PathBuf::from(ast_match.file.clone()));
        Symbol {
            name: ast_match.text.clone(),
            kind: ast_match.rule_id.clone(),
            identifier_position: FilePosition {
                path: path.clone(),
                position: Position {
                    line: ast_match.range.start.line as u32,
                    character: ast_match.range.start.column as u32,
                },
            },
            range: FileRange {
                path: path.clone(),
                start: Position {
                    line: ast_match
                        .meta_variables
                        .multi
                        .secondary
                        .last()
                        .expect("Expected at least one secondary variable")
                        .range
                        .start
                        .line as u32,
                    // character: ast_match
                    //     .meta_variables
                    //     .multi
                    //     .secondary
                    //     .last()
                    //     .unwrap()
                    //     .range
                    //     .start
                    //     .column as u32,
                    character: 0, // TODO: this is not technically true, we're returning the whole line for consistency
                },
                end: Position {
                    line: ast_match
                        .meta_variables
                        .multi
                        .secondary
                        .last()
                        .expect("Expected at least one secondary variable")
                        .range
                        .end
                        .line as u32,
                    character: ast_match
                        .meta_variables
                        .multi
                        .secondary
                        .last()
                        .expect("Expected at least one secondary variable")
                        .range
                        .end
                        .column as u32,
                },
            },
        }
    }
}
