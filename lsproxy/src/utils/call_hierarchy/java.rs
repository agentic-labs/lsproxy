use super::LanguageCallHierarchy;
use lsp_types::SymbolKind;
use tree_sitter_java;

pub struct JavaCallHierarchy {}

impl LanguageCallHierarchy for JavaCallHierarchy {
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

    fn is_function_type(&self, node_type: &str) -> bool {
        matches!(
            node_type,
            "method_declaration" | "constructor_declaration" | "method_invocation" | "object_creation_expression"
        )
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

    fn is_call_node(&self, node_type: &str) -> bool {
        matches!(node_type, "method_invocation" | "object_creation_expression")
    }
}