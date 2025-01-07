use super::LanguageCallHierarchy;
use lsp_types::SymbolKind;
use tree_sitter_php;

pub struct PhpCallHierarchy {}

impl LanguageCallHierarchy for PhpCallHierarchy {
    fn get_definition_node_at_position<'a>(&self, node: &'a tree_sitter::Node<'a>) -> Option<tree_sitter::Node<'a>> {
        if node.kind() == "visibility_modifier" {
            // For PHP method declarations, navigate to the method name
            let parent = node.parent()?;
            if parent.kind() == "method_declaration" {
                // Find the name node within the method declaration
                for child in 0..parent.child_count() {
                    if let Some(child_node) = parent.child(child) {
                        if child_node.kind() == "name" {
                            return Some(child_node);
                        }
                    }
                }
            }
        }
        Some(node.clone())
    }

    fn get_function_call_query(&self) -> &'static str {
        r#"
            ; Function calls
            (function_call_expression
                function: (name) @func_name)

            ; Method calls
            (member_call_expression
                name: (name) @func_name)

            ; Static method calls
            (scoped_call_expression
                name: (name) @func_name)
        "#
    }

    fn get_function_definition_query(&self) -> &'static str {
        r#"
            ; Regular functions
            (function_definition
                name: (name) @func_name)

            ; Class methods (including static)
            (method_declaration
                name: (name) @func_name)
        "#
    }

    fn get_enclosing_function_pattern(&self) -> &'static str {
        "(function_definition | method_declaration | class_declaration) @cap"
    }

    fn determine_symbol_kind(&self, node_type: &str, node_text: &str) -> SymbolKind {
        match node_type {
            "class_declaration" => SymbolKind::CLASS,
            "method_declaration" => SymbolKind::METHOD,
            _ => SymbolKind::FUNCTION,
        }
    }

    fn configure_parser(&self, parser: &mut tree_sitter::Parser) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        parser.set_language(&tree_sitter_php::LANGUAGE_PHP.into())?;
        Ok(())
    }

    fn is_package_root(&self, dir: &std::path::Path) -> bool {
        dir.join("composer.json").exists() || dir.join("main.php").exists()
    }

    fn is_identifier_node(&self, node_type: &str) -> bool {
        node_type == "name"
    }

    fn is_callable_type(&self, node_type: &str) -> bool {
        matches!(node_type, 
            // Definitions
            "function_definition" | 
            "method_declaration" |
            // Calls
            "function_call_expression" |
            "member_call_expression" |
            "scoped_call_expression"
        )
    }

    fn is_definition(&self, node_type: &str) -> bool {
        matches!(node_type,
            "function_definition" |
            "method_declaration" |
            "class_declaration"  // included because it can contain method definitions
        )
    }
}