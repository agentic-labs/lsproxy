use super::LanguageCallHierarchy;
use lsp_types::SymbolKind;
use tree_sitter_cpp;

pub struct CppCallHierarchy {}

impl LanguageCallHierarchy for CppCallHierarchy {
    fn get_function_call_query(&self) -> &'static str {
        r#"
        [
          (call_expression
            function: [(identifier) (field_identifier)] @func_name)
          (call_expression
            function: (field_expression
              field: (field_identifier) @func_name))
        ] @call
        "#
    }

    fn get_function_definition_query(&self) -> &'static str {
        r#"
        [
          (function_definition
            declarator: (function_declarator
              declarator: [(identifier) (field_identifier)] @func_name))
          (function_definition
            declarator: (function_declarator
              declarator: (operator_name) @func_name))
        ] @func_decl
        "#
    }

    fn is_function_type(&self, node_type: &str) -> bool {
        matches!(
            node_type,
            "function_definition" | 
            "function_declarator" | 
            "template_declaration"
        )
    }

    fn get_enclosing_function_pattern(&self) -> &'static str {
        "(function_definition | class_specifier) @cap"
    }

    fn determine_symbol_kind(&self, node_type: &str, node_text: &str) -> SymbolKind {
        match node_type {
            "class_specifier" => SymbolKind::CLASS,
            _ if node_text.contains("operator") => SymbolKind::OPERATOR,
            _ if node_text.contains("~") => SymbolKind::CONSTRUCTOR,
            _ if node_text.starts_with("::") || node_text.contains("::") => SymbolKind::METHOD,
            _ => SymbolKind::FUNCTION,
        }
    }

    fn configure_parser(&self, parser: &mut tree_sitter::Parser) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        parser.set_language(&tree_sitter_cpp::LANGUAGE.into())?;
        Ok(())
    }

    fn is_package_root(&self, dir: &std::path::Path) -> bool {
        dir.join("CMakeLists.txt").exists() || 
        dir.join("compile_commands.json").exists() ||
        dir.join("Makefile").exists() ||
        dir.join("build").exists()
    }
}