pub trait LanguageCallHierarchy {
    fn get_function_call_query(&self) -> &'static str;
    fn get_function_definition_query(&self) -> &'static str;
    fn is_function_type(&self, node_type: &str) -> bool;
    fn get_enclosing_function_pattern(&self) -> &'static str;
    fn determine_symbol_kind(&self, node_type: &str, node_text: &str) -> lsp_types::SymbolKind;
    fn configure_parser(&self, parser: &mut tree_sitter::Parser) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn is_package_root(&self, dir: &std::path::Path) -> bool;
}

pub mod python;
pub mod typescript;
pub mod rust;
mod registry;

pub fn get_call_hierarchy_handler(language: &str) -> Option<Box<dyn LanguageCallHierarchy>> {
    registry::get_handler(language)
}