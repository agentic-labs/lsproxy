use super::LanguageCallHierarchy;
use lsp_types::SymbolKind;

pub struct TypeScriptCallHierarchy {}

impl LanguageCallHierarchy for TypeScriptCallHierarchy {
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

    fn is_function_type(&self, node_type: &str) -> bool {
        matches!(
            node_type,
            "function_declaration" | "method_definition" | "arrow_function"
        )
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
}