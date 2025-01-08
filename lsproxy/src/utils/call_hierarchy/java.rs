use super::LanguageCallHierarchy;
use lsp_types::SymbolKind;
use tree_sitter_java;

pub struct JavaCallHierarchy {}

impl LanguageCallHierarchy for JavaCallHierarchy {
    fn get_call_name_node<'a>(&self, call_node: &'a tree_sitter::Node<'a>) -> Option<tree_sitter::Node<'a>> {
        match call_node.kind() {
            // Method invocations
            "method_invocation" => {
                // Try to get the name directly
                call_node.child_by_field_name("name")
                    .or_else(|| {
                        // If no direct name, try to get it from the object
                        call_node.child_by_field_name("object")
                            .and_then(|obj| obj.child_by_field_name("name"))
                    })
            },
            // Constructor calls
            "object_creation_expression" => call_node.child_by_field_name("type"),
            // Field access
            "field_access" => call_node.child_by_field_name("field"),
            // Generic fallback
            _ => None
        }
    }

    fn get_definition_node_at_position<'a>(&self, node: &'a tree_sitter::Node<'a>) -> Option<tree_sitter::Node<'a>> {
        match node.kind() {
            "expression_statement" => {
                // For expression statements, try to get the method invocation
                if let Some(method_call) = node.child(0) {
                    if method_call.kind() == "method_invocation" {
                        // For a direct method call, get the identifier
                        if let Some(identifier) = method_call.child_by_field_name("name") {
                            return Some(identifier);
                        }
                    }
                }
                Some(node.clone());
            },
            "block" => {
                // For blocks, find the most specific node at our position
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "expression_statement" {
                        if let Some(method_call) = child.child(0) {
                            if method_call.kind() == "method_invocation" {
                                // For a direct method call, get the identifier
                                if let Some(identifier) = method_call.child_by_field_name("name") {
                                    return Some(identifier);
                                }
                            }
                        }
                    }
                }
                Some(node.clone());
            },
            "public" | "private" | "protected" | "static" | "final" => {
                // For Java method declarations, navigate to the method name
                let parent = node.parent()?;
                if parent.kind() == "method_declaration" || parent.kind() == "constructor_declaration" {
                    // Find the identifier node within the method declaration
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
        ; Method calls (including on local variables)
        (method_invocation
          name: (identifier) @func_name) @call

        ; Constructor calls
        (object_creation_expression
          type: (type_identifier) @class_name) @call
        "#
    }

    fn get_function_definition_query(&self) -> &'static str {
        r#"
        ; Regular method declarations
        (method_declaration
          name: (identifier) @func_name) @func_decl

        ; Constructor declarations
        (constructor_declaration
          name: (identifier) @func_name) @func_decl

        ; Class declarations with methods
        (class_declaration
          name: (identifier) @class_name
          body: (class_body
            (method_declaration
              name: (identifier) @func_name) @func_decl))

        ; Interface method declarations
        (interface_declaration
          body: (interface_body
            (method_declaration
              name: (identifier) @func_name) @func_decl))
        "#
    }

    fn get_enclosing_function_pattern(&self) -> &'static str {
        "(method_declaration | constructor_declaration | class_declaration | interface_declaration) @cap"
    }

    fn determine_symbol_kind(&self, node_type: &str, node_text: &str) -> SymbolKind {
        match node_type {
            "class_declaration" => SymbolKind::CLASS,
            "interface_declaration" => SymbolKind::INTERFACE,
            "constructor_declaration" => SymbolKind::CONSTRUCTOR,
            _ if node_text.contains("static ") => SymbolKind::FUNCTION,
            _ => SymbolKind::METHOD,
        }
    }

    fn configure_parser(&self, parser: &mut tree_sitter::Parser) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        parser.set_language(&tree_sitter_java::LANGUAGE.into())?;
        Ok(())
    }

    fn is_package_root(&self, dir: &std::path::Path) -> bool {
        // Java projects typically have either pom.xml (Maven) or build.gradle (Gradle)
        dir.join("pom.xml").exists() || dir.join("build.gradle").exists() || dir.join("Main.java").exists()
    }

    fn is_identifier_node(&self, node_type: &str) -> bool {
        matches!(node_type, "identifier" | "type_identifier")
    }

    fn is_callable_type(&self, node_type: &str) -> bool {
        matches!(node_type,
            // Definitions
            "method_declaration" |
            "constructor_declaration" |
            // Calls
            "method_invocation" |
            "object_creation_expression" |
            // Lambda expressions
            "lambda_expression"
        )
    }

    fn is_definition(&self, node_type: &str) -> bool {
        matches!(node_type,
            "method_declaration" |
            "constructor_declaration" |
            "class_declaration" |  // included because it can contain methods
            "interface_declaration" |  // included because it can contain method signatures
            "enum_declaration" |  // enums can have methods too
            "annotation_type_declaration"  // annotations can have default methods
        )
    }
}