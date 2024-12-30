use super::LanguageCallHierarchy;
use lsp_types::SymbolKind;
use tree_sitter_python;

pub struct PythonCallHierarchy {}

impl LanguageCallHierarchy for PythonCallHierarchy {
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

    fn is_function_type(&self, node_type: &str) -> bool {
        matches!(node_type, "function_definition")
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
}