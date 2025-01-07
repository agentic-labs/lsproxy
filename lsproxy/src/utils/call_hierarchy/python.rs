use super::LanguageCallHierarchy;
use lsp_types::SymbolKind;
use tree_sitter_python;

pub struct PythonCallHierarchy {}

impl LanguageCallHierarchy for PythonCallHierarchy {
    fn get_call_name_node<'a>(&self, call_node: &'a tree_sitter::Node<'a>) -> Option<tree_sitter::Node<'a>> {
        match call_node.kind() {
            // Function and method calls
            "call" => {
                let func = call_node.child_by_field_name("function")?;
                if func.kind() == "attribute" {
                    // Method call (obj.method())
                    func.child_by_field_name("attribute")
                } else {
                    // Regular function call
                    Some(func)
                }
            },
            // Generic fallback
            _ => None
        }
    }

    fn get_definition_node_at_position<'a>(&self, node: &'a tree_sitter::Node<'a>) -> Option<tree_sitter::Node<'a>> {
        match node.kind() {
            "decorator" => {
                // For Python decorators, navigate to the decorated function name
                let parent = node.parent()?;
                if parent.kind() == "decorated_definition" {
                    // Find the function_definition within the decorated_definition
                    for child in 0..parent.child_count() {
                        if let Some(child_node) = parent.child(child) {
                            if child_node.kind() == "function_definition" {
                                // Find the identifier within the function_definition
                                for func_child in 0..child_node.child_count() {
                                    if let Some(func_child_node) = child_node.child(func_child) {
                                        if func_child_node.kind() == "identifier" {
                                            return Some(func_child_node);
                                        }
                                    }
                                }
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
            ; Any function or method call
            (call) @call
        "#
    }

    fn get_function_definition_query(&self) -> &'static str {
        r#"
            ; Regular functions
            (function_definition
              name: (identifier) @func_name
            ) @func_decl

            ; Class methods
            (class_definition
              name: (identifier) @class_name
              body: (block 
                (function_definition
                  name: (identifier) @func_name) @func_decl)
            )

            ; Async functions
            (function_definition
              "async"
              name: (identifier) @func_name
            ) @func_decl
        "#
    }

    fn get_enclosing_function_pattern(&self) -> &'static str {
        "(function_definition | class_definition) @cap"
    }

    fn determine_symbol_kind(&self, node_type: &str, node_text: &str) -> SymbolKind {
        match node_type {
            "class_definition" => SymbolKind::CLASS,
            _ if node_text.contains("self") => SymbolKind::METHOD,
            _ => SymbolKind::FUNCTION,
        }
    }

    fn configure_parser(&self, parser: &mut tree_sitter::Parser) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        parser.set_language(&tree_sitter_python::LANGUAGE.into())?;
        Ok(())
    }

    fn is_package_root(&self, dir: &std::path::Path) -> bool {
        dir.join("__init__.py").exists()
    }

    fn is_identifier_node(&self, node_type: &str) -> bool {
        node_type == "identifier"
    }

    fn is_callable_type(&self, node_type: &str) -> bool {
        matches!(node_type,
            // Definitions
            "function_definition" |
            "lambda" |
            // Calls
            "call"
        )
    }

    fn is_definition(&self, node_type: &str) -> bool {
        matches!(node_type,
            "function_definition" |
            "class_definition" |  // included because it can contain method definitions
            "decorated_definition"  // for decorated functions/classes
        )
    }
}