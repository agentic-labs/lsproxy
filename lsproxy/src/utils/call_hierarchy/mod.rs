use log::debug;

pub trait LanguageCallHierarchy: Send + Sync {
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

    /// Get the name node from a call node based on the language's AST structure.
    /// Each language should implement this based on its AST structure.
    /// Default implementation tries common field names.
    fn get_call_name_node<'a>(&self, call_node: &'a tree_sitter::Node<'a>) -> Option<tree_sitter::Node<'a>> {
        // Try common field names as a fallback
        call_node.child_by_field_name("function")
            .or_else(|| call_node.child_by_field_name("name"))
            .or_else(|| call_node.child_by_field_name("identifier"))
    }

    /// Find all function/method calls within a method body.
    /// Default implementation does a DFS traversal of the AST looking for callable nodes.
    /// Languages can override this if they need special handling of method bodies.
    fn find_calls_in_method_body<'a>(&self, method_node: &'a tree_sitter::Node<'a>, source: &'a [u8]) -> Vec<tree_sitter::Node<'a>> {
        let mut calls = Vec::new();
        let mut stack = vec![method_node.clone()];
        
        while let Some(current) = stack.pop() {
            // Add all children to the stack for DFS traversal
            let mut child_cursor = current.walk();
            for child in current.children(&mut child_cursor) {
                debug!("Examining child node: kind={}, text={:?}",
                    child.kind(),
                    child.utf8_text(source).unwrap_or("<invalid utf8>"));
                stack.push(child);

                // Check if current node is a call
                if self.is_callable_type(child.kind()) && !self.is_definition(child.kind()) {
                    if let Some(name_node) = self.get_call_name_node(&child) {
                        debug!("Found call in method body: kind={}, name={:?}", 
                            child.kind(),
                            name_node.utf8_text(source).unwrap_or("<invalid utf8>"));
                        calls.push(child);
                    }
                }
            }
        }
        
        debug!("Found {} total calls in method body", calls.len());
        calls
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