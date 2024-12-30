use super::LanguageCallHierarchy;
use lsp_types::SymbolKind;

pub struct RustCallHierarchy {}

impl LanguageCallHierarchy for RustCallHierarchy {
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

    fn is_function_type(&self, node_type: &str) -> bool {
        matches!(
            node_type,
            "function_item" | "closure_expression" | "impl_item" | "identifier"
        )
    }

    fn get_enclosing_function_pattern(&self) -> &'static str {
        "(function_item | impl_item | closure_expression | identifier) @cap"
    }

    fn determine_symbol_kind(&self, node_type: &str, node_text: &str) -> SymbolKind {
        match node_type {
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
}