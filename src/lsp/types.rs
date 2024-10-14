use lsp_types::{GotoDefinitionResponse, Location, Url};
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use strum_macros::{Display, EnumString};

pub const MOUNT_DIR: &str = "/mnt/repo";

#[derive(
    Debug,
    EnumString,
    Display,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    utoipa::ToSchema,
)]
#[strum(serialize_all = "lowercase")]
pub enum SupportedLSP {
    Python,
    TypeScriptJavaScript,
    Rust,
}

#[derive(Debug, Clone)]
pub struct UniqueDefinition {
    pub uri: Url,
    pub range_start: (u32, u32),
    pub range_end: (u32, u32),
    pub original: GotoDefinitionResponse,
}

impl Hash for UniqueDefinition {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.uri.hash(state);
        self.range_start.hash(state);
        self.range_end.hash(state);
    }
}

impl PartialEq for UniqueDefinition {
    fn eq(&self, other: &Self) -> bool {
        self.uri == other.uri
            && self.range_start == other.range_start
            && self.range_end == other.range_end
    }
}

impl Eq for UniqueDefinition {}

impl From<GotoDefinitionResponse> for UniqueDefinition {
    fn from(response: GotoDefinitionResponse) -> Self {
        match response.clone() {
            GotoDefinitionResponse::Scalar(location) => Self::from_location(location, response),
            GotoDefinitionResponse::Array(locations) if !locations.is_empty() => {
                Self::from_location(locations[0].clone(), response)
            }
            GotoDefinitionResponse::Link(links) if !links.is_empty() => {
                let location = Location::new(links[0].target_uri.clone(), links[0].target_range);
                Self::from_location(location, response)
            }
            _ => panic!("Unexpected empty GotoDefinitionResponse"),
        }
    }
}

impl UniqueDefinition {
    fn from_location(location: Location, original: GotoDefinitionResponse) -> Self {
        UniqueDefinition {
            uri: location.uri,
            range_start: (location.range.start.line, location.range.start.character),
            range_end: (location.range.end.line, location.range.end.character),
            original,
        }
    }
}