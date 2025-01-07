pub trait LanguageCallHierarchy {
    fn get_function_call_query(&self) -> &'static str;
    fn get_function_definition_query(&self) -> &'static str;
    fn get_enclosing_function_pattern(&self) -> &'static str;
    fn determine_symbol_kind(&self, node_type: &str, node_text: &str) -> lsp_types::SymbolKind;
    fn configure_parser(&self, parser: &mut tree_sitter::Parser) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn is_package_root(&self, dir: &std::path::Path) -> bool;
    fn is_identifier_node(&self, node_type: &str) -> bool;
    
    /// Identifies if a node represents a callable entity (function, method, or closure)
    /// This includes both definitions and call sites
    fn is_callable_type(&self, node_type: &str) -> bool;

    /// Identifies if a node represents a definition (as opposed to a reference/call)
    /// This includes function definitions, method definitions, and class definitions
    fn is_definition(&self, node_type: &str) -> bool;

    /// Optional: Only needed for languages that distinguish declarations from definitions
    fn is_declaration(&self, node_type: &str) -> bool {
        false // Default implementation returns false
    }
    
    // Default implementation for getting the definition node at a position
    fn get_definition_node_at_position<'a>(&self, node: &'a tree_sitter::Node<'a>) -> Option<tree_sitter::Node<'a>> {
        Some(node.clone())
    }
}

pub mod python;
pub mod typescript;
pub mod rust;
pub mod go;
pub mod java;
pub mod cpp;
pub mod php;
mod registry;

pub fn get_call_hierarchy_handler(language: &str) -> Option<Box<dyn LanguageCallHierarchy>> {
    registry::get_handler(language)
}