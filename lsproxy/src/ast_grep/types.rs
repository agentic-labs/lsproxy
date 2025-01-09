use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    api_types::{FilePosition, FileRange, Identifier, Position, Symbol},
    utils::file_utils::absolute_path_to_relative_path_string,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AstGrepMatch {
    pub text: String,
    pub range: AstGrepRange,
    pub file: String,
    pub lines: String,
    pub char_count: CharCount,
    pub language: String,
    pub meta_variables: MetaVariables,
    pub rule_id: String,
    pub labels: Option<Vec<Label>>,
}

impl AstGrepMatch {
    pub fn get_source_code(&self) -> String {
        if let Some(context) = &self.meta_variables.single.context {
            context.text.clone()
        } else {
            self.text.clone()
        }
    }

    pub fn get_range(&self) -> AstGrepRange {
        if let Some(context) = &self.meta_variables.single.context {
            context.range.clone()
        } else {
            self.range.clone()
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AstGrepRange {
    pub byte_offset: ByteOffset,
    pub start: AstGrepPosition,
    pub end: AstGrepPosition,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ByteOffset {
    pub start: usize,
    pub end: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AstGrepPosition {
    pub line: u32,
    pub column: u32,
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
    pub single: SingleVariable,
    pub multi: MultiVariables,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SingleVariable {
    #[serde(rename = "NAME")]
    pub name: MetaVariable,
    #[serde(rename = "CONTEXT")]
    pub context: Option<MetaVariable>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MultiVariables {
    pub secondary: Option<Vec<MetaVariable>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MetaVariable {
    pub text: String,
    pub range: AstGrepRange,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    pub text: String,
    pub range: AstGrepRange,
}

impl From<&AstGrepMatch> for lsp_types::Position {
    fn from(ast_match: &AstGrepMatch) -> Self {
        Self {
            line: ast_match.range.start.line as u32,
            character: ast_match.range.start.column as u32,
        }
    }
}

impl From<AstGrepMatch> for Symbol {
    fn from(ast_match: AstGrepMatch) -> Self {
        assert!(ast_match.rule_id != "all-identifiers");
        let path = absolute_path_to_relative_path_string(&PathBuf::from(ast_match.file.clone()));
        let match_range = ast_match.get_range();
        Symbol {
            name: ast_match.meta_variables.single.name.text.clone(),
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
                    line: match_range.start.line as u32,
                    character: 0, // TODO: this is not technically true, we're returning the whole line for consistency
                },
                end: Position {
                    line: match_range.end.line as u32,
                    character: match_range.end.column as u32,
                },
            },
        }
    }
}

impl From<AstGrepMatch> for Identifier {
    fn from(ast_match: AstGrepMatch) -> Self {
        let path = absolute_path_to_relative_path_string(&PathBuf::from(ast_match.file.clone()));
        let match_range = ast_match.get_range();
        Identifier {
            name: ast_match.meta_variables.single.name.text.clone(),
            range: FileRange {
                path: path.clone(),
                start: Position {
                    line: match_range.start.line as u32,
                    character: match_range.start.column as u32,
                },
                end: Position {
                    line: match_range.end.line as u32,
                    character: match_range.end.column as u32,
                },
            },
        }
    }
}
