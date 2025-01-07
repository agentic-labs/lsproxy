use super::LanguageCallHierarchy;
use lsp_types::SymbolKind;

pub struct RustCallHierarchy {}

impl LanguageCallHierarchy for RustCallHierarchy {
    fn get_definition_node_at_position<'a>(&self, node: &'a tree_sitter::Node<'a>) -> Option<tree_sitter::Node<'a>> {
        match node.kind() {
            "attribute" | "attribute_item" => {
                // For Rust attributes, navigate to the attributed function name
                let parent = node.parent()?;
                if parent.kind() == "function_item" {
                    // Find the identifier within the function_item
                    for child in 0..parent.child_count() {
                        if let Some(child_node) = parent.child(child) {
                            if child_node.kind() == "identifier" {
                                return Some(child_node);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        Some(node.clone())
    }
    fn get_function_call_query(&self) -> &'static str {
        r#"
        ; Regular function calls
        (call_expression
          function: (identifier) @func_name) @call

        ; Method calls
        (call_expression
          function: (field_expression
            field: (field_identifier) @func_name)) @call

        ; Associated function calls (static methods)
        (call_expression
          function: (scoped_identifier
            path: (identifier) @type_name
            name: (identifier) @func_name)) @call
        "#
    }

    fn get_function_definition_query(&self) -> &'static str {
        r#"[
          (function_item
            name: (identifier) @func_name) @func_decl
          (impl_item
            body: (declaration_list
              (function_item
                name: (identifier) @func_name))) @func_decl
        ]"#
    }

    fn get_enclosing_function_pattern(&self) -> &'static str {
        "(function_item | impl_item | closure_expression | identifier) @cap"
    }

    fn determine_symbol_kind(&self, node_type: &str, node_text: &str) -> SymbolKind {
        match node_type {
            "impl_item" => SymbolKind::CLASS,  // impl blocks are similar to classes
            "trait_item" => SymbolKind::INTERFACE,
            _ if node_text.contains("&self") || node_text.contains("&mut self") => SymbolKind::METHOD,
            _ => SymbolKind::FUNCTION,
        }
    }

    fn configure_parser(&self, parser: &mut tree_sitter::Parser) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        parser.set_language(&tree_sitter_rust::LANGUAGE.into())?;
        Ok(())
    }

    fn is_package_root(&self, dir: &std::path::Path) -> bool {
        dir.join("Cargo.toml").exists()
    }

    fn is_identifier_node(&self, node_type: &str) -> bool {
        matches!(node_type, "identifier" | "field_identifier")
    }

    fn is_callable_type(&self, node_type: &str) -> bool {
        matches!(node_type,
            // Definitions
            "function_item" |
            "closure_expression" |
            // Calls
            "call_expression" |
            "method_call_expression"
        )
    }

    fn is_definition(&self, node_type: &str) -> bool {
        matches!(node_type,
            "function_item" |
            "impl_item" |  // impl blocks can contain methods
            "trait_item" |  // trait definitions can contain method signatures
            "closure_expression" |
            "macro_definition"  // macros can expand to functions
        )
    }
}