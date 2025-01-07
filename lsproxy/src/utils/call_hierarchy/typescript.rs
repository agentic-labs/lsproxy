use super::LanguageCallHierarchy;
use lsp_types::SymbolKind;
use tree_sitter_typescript;

pub struct TypeScriptCallHierarchy {}

impl LanguageCallHierarchy for TypeScriptCallHierarchy {
    fn get_definition_node_at_position<'a>(&self, node: &'a tree_sitter::Node<'a>) -> Option<tree_sitter::Node<'a>> {
        match node.kind() {
            "public" | "private" | "protected" | "static" | "readonly" | "async" => {
                // For TypeScript method declarations, navigate to the method name
                let parent = node.parent()?;
                if parent.kind() == "method_definition" {
                    // Find the name node within the method definition
                    for child in 0..parent.child_count() {
                        if let Some(child_node) = parent.child(child) {
                            if matches!(child_node.kind(), "property_identifier" | "identifier") {
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
              function: (member_expression
                property: (property_identifier) @func_name)) @call

            ; Constructor calls
            (new_expression
              constructor: (identifier) @class_name) @call

            ; Static method calls
            (call_expression
              function: (member_expression
                object: (identifier) @class_name
                property: (property_identifier) @func_name)) @call
        "#
    }

    fn get_function_definition_query(&self) -> &'static str {
        r#"
            ; Regular functions
            (function_declaration
              name: (identifier) @func_name
            ) @func_decl

            ; Class methods (including constructors and shorthand methods)
            (method_definition
              name: (_) @func_name
            ) @func_decl

            ; Class declarations with methods
            (class_declaration
              name: (type_identifier) @class_name
              body: (class_body
                (method_definition
                  name: (_) @func_name) @func_decl)
            )

            ; Arrow functions with names (variable declarations)
            (variable_declarator
              name: (identifier) @func_name
              value: (arrow_function)) @func_decl
        "#
    }

    fn get_enclosing_function_pattern(&self) -> &'static str {
        "(function_declaration | method_definition | class_declaration) @cap"
    }

    fn determine_symbol_kind(&self, node_type: &str, node_text: &str) -> SymbolKind {
        match node_type {
            "class_declaration" => SymbolKind::CLASS,
            _ if node_text.contains("this") => SymbolKind::METHOD,
            _ => SymbolKind::FUNCTION,
        }
    }

    fn configure_parser(&self, parser: &mut tree_sitter::Parser) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        parser.set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())?;
        Ok(())
    }

    fn is_package_root(&self, dir: &std::path::Path) -> bool {
        dir.join("package.json").exists()
    }

    fn is_identifier_node(&self, node_type: &str) -> bool {
        matches!(node_type, "identifier" | "property_identifier" | "type_identifier")
    }

    fn is_callable_type(&self, node_type: &str) -> bool {
        matches!(node_type,
            // Definitions
            "function_declaration" |
            "method_definition" |
            "arrow_function" |
            // Calls
            "call_expression" |
            "new_expression"
        )
    }

    fn is_definition(&self, node_type: &str) -> bool {
        matches!(node_type,
            "function_declaration" |
            "method_definition" |
            "class_declaration" |  // included because it can contain method definitions
            "arrow_function" |
            "variable_declarator"  // for arrow functions assigned to variables
        )
    }
}