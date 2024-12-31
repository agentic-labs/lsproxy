use super::LanguageCallHierarchy;
use lsp_types::SymbolKind;

pub struct GoCallHierarchy {}

impl LanguageCallHierarchy for GoCallHierarchy {
    fn get_function_call_query(&self) -> &'static str {
        r#"
        ; Regular function calls
        (call_expression
          function: (identifier) @func_name) @call

        ; Method calls
        (call_expression
          function: (selector_expression
            field: (field_identifier) @func_name)) @call

        ; Function calls through package
        (call_expression
          function: (selector_expression
            operand: (identifier) @pkg_name
            field: (field_identifier) @func_name)) @call
        "#
    }

    fn get_function_definition_query(&self) -> &'static str {
        r#"
        ; Regular function declarations
        (function_declaration
          name: (identifier) @func_name) @func_decl

        ; Method declarations
        (method_declaration
          name: (field_identifier) @func_name
          receiver: (parameter_list) @receiver) @func_decl
        "#
    }

    fn is_function_type(&self, node_type: &str) -> bool {
        matches!(
            node_type,
            "function_declaration" | "method_declaration"
        )
    }

    fn get_enclosing_function_pattern(&self) -> &'static str {
        "(function_declaration | method_declaration) @cap"
    }

    fn determine_symbol_kind(&self, node_type: &str, _node_text: &str) -> SymbolKind {
        match node_type {
            "method_declaration" => SymbolKind::METHOD,
            _ => SymbolKind::FUNCTION,
        }
    }

    fn configure_parser(&self, parser: &mut tree_sitter::Parser) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        parser.set_language(&tree_sitter_go::LANGUAGE.into())?;
        Ok(())
    }

    fn is_package_root(&self, dir: &std::path::Path) -> bool {
        dir.join("go.mod").exists()
    }
}