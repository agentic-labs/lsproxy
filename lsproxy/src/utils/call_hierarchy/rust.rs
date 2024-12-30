use super::LanguageCallHierarchy;
use lsp_types::SymbolKind;

pub struct RustCallHierarchy {}

impl LanguageCallHierarchy for RustCallHierarchy {
    fn get_function_call_query(&self) -> &'static str {
        r#"
        ; Function and method calls
        (call_expression
            function: [
                (identifier) @func_name
                (field_expression
                    field: (field_identifier) @func_name)
            ]
        ) @call

        ; Associated function calls (like String::new())
        (call_expression
            function: (scoped_identifier
                name: (identifier) @func_name)
        ) @call
        "#
    }

    fn get_function_definition_query(&self) -> &'static str {
        r#"
        ; Regular functions
        (function_item
            name: (identifier) @func_name
        ) @func_decl

        ; Methods in impl blocks
        (impl_item
            name: (identifier) @func_name
        ) @func_decl

        ; Associated functions in impl blocks
        (impl_block
            type: (type_identifier) @type_name
            body: (declaration_list
                (function_item
                    name: (identifier) @func_name) @func_decl)
        )
        "#
    }

    fn is_function_type(&self, node_type: &str) -> bool {
        matches!(
            node_type,
            "function_item" | "impl_item"
        )
    }

    fn get_enclosing_function_pattern(&self) -> &'static str {
        "(function_item | impl_item | impl_block) @cap"
    }

    fn determine_symbol_kind(&self, node_type: &str, node_text: &str) -> SymbolKind {
        match node_type {
            "impl_item" => SymbolKind::METHOD,
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