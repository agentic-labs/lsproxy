use super::LanguageCallHierarchy;

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
}