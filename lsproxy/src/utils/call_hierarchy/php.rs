use super::LanguageCallHierarchy;
use log::debug;
use lsp_types::SymbolKind;
use tree_sitter_php;

pub struct PhpCallHierarchy {}

impl LanguageCallHierarchy for PhpCallHierarchy {
    fn get_definition_node_at_position<'a>(&self, node: &'a tree_sitter::Node<'a>) -> Option<tree_sitter::Node<'a>> {
        if node.kind() == "visibility_modifier" {
            // For PHP method declarations, navigate to the method declaration node
            let parent = node.parent()?;
            if parent.kind() == "method_declaration" {
                return Some(parent);
            }
        }
        Some(node.clone())
    }

    fn get_call_name_node<'a>(&self, call_node: &'a tree_sitter::Node<'a>) -> Option<tree_sitter::Node<'a>> {
        debug!("get_call_name_node: Processing node kind={}", call_node.kind());
        let result = match call_node.kind() {
            // Method calls with $this
            "member_call_expression" => {
                debug!("Processing member_call_expression");
                let name = call_node.child_by_field_name("name");
                debug!("Found name node: {:?}", name.map(|n| n.kind()));
                // Get the actual name text node
                name.and_then(|n| {
                    if n.kind() == "name" {
                        Some(n)
                    } else {
                        None
                    }
                })
            },
            // Function calls
            "function_call_expression" => {
                debug!("Processing function_call_expression");
                let func = call_node.child_by_field_name("function");
                debug!("Found function node: {:?}", func.map(|n| n.kind()));
                if let Some(f) = func {
                    match f.kind() {
                        "member_expression" => {
                            let prop = f.child_by_field_name("property");
                            debug!("Found member expression property: {:?}", prop.map(|n| n.kind()));
                            prop.and_then(|p| {
                                if p.kind() == "name" {
                                    Some(p)
                                } else {
                                    None
                                }
                            })
                        },
                        "name" => Some(f),
                        _ => None
                    }
                } else {
                    None
                }
            },
            // Object creation
            "object_creation_expression" => {
                debug!("Processing object_creation_expression");
                let class = call_node.child_by_field_name("class");
                debug!("Found class node: {:?}", class.map(|n| n.kind()));
                class.and_then(|c| {
                    if c.kind() == "name" {
                        Some(c)
                    } else {
                        None
                    }
                })
            },
            // Static method calls
            "scoped_call_expression" => {
                debug!("Processing scoped_call_expression");
                let name = call_node.child_by_field_name("name");
                debug!("Found name node: {:?}", name.map(|n| n.kind()));
                name.and_then(|n| {
                    if n.kind() == "name" {
                        Some(n)
                    } else {
                        None
                    }
                })
            },
            // Generic fallback
            _ => {
                debug!("Unhandled node type: {}", call_node.kind());
                None
            }
        };
        debug!("get_call_name_node: Returning node: {:?}", result.map(|n| n.kind()));
        result
    }

    fn find_calls_in_method_body<'a>(&self, method_node: &'a tree_sitter::Node<'a>, source: &'a [u8]) -> Vec<tree_sitter::Node<'a>> {
        let mut calls = Vec::new();
        
        debug!("find_calls_in_method_body: Starting with node kind={}, text={:?}",
            method_node.kind(),
            method_node.utf8_text(source).unwrap_or("<invalid utf8>"));
        
        // For PHP methods, we need to find the compound_statement (method body)
        let body_node = if method_node.kind() == "method_declaration" {
            debug!("Finding compound_statement in method_declaration");
            // Find the compound_statement node
            let mut cursor = method_node.walk();
            let mut body = None;
            debug!("Method declaration has {} children", method_node.child_count());
            for child in method_node.children(&mut cursor) {
                debug!("Checking child node: kind={}, text={:?}",
                    child.kind(),
                    child.utf8_text(source).unwrap_or("<invalid utf8>"));
                if child.kind() == "compound_statement" {
                    debug!("Found compound_statement node");
                    body = Some(child);
                    break;
                }
            }
            body.unwrap_or_else(|| {
                debug!("No compound_statement found, using method_node");
                method_node.clone()
            })
        } else {
            debug!("Not a method_declaration, using node as is");
            method_node.clone()
        };

        debug!("Starting call search in method body: kind={}, text={:?}", 
            body_node.kind(),
            body_node.utf8_text(source).unwrap_or("<invalid utf8>"));

        let mut stack = vec![body_node];
        
        while let Some(current) = stack.pop() {
            // Add all children to the stack for DFS traversal
            let mut child_cursor = current.walk();
            debug!("Processing node: kind={}, text={:?}, child_count={}",
                current.kind(),
                current.utf8_text(source).unwrap_or("<invalid utf8>"),
                current.child_count());
            
            for child in current.children(&mut child_cursor) {
                debug!("Examining child node: kind={}, text={:?}",
                    child.kind(),
                    child.utf8_text(source).unwrap_or("<invalid utf8>"));
                
                // Push child first to ensure we traverse the entire tree
                stack.push(child);

                // Check if current node is a call (not a definition)
                if self.is_callable_type(child.kind()) && !self.is_definition(child.kind()) {
                    debug!("Found callable node: kind={}, text={:?}",
                        child.kind(),
                        child.utf8_text(source).unwrap_or("<invalid utf8>"));
                    
                    if let Some(name_node) = self.get_call_name_node(&child) {
                        // Only include if it's a name node (not a property access)
                        if name_node.kind() == "name" {
                            debug!("Found call in method body: kind={}, name={:?}", 
                                child.kind(),
                                name_node.utf8_text(source).unwrap_or("<invalid utf8>"));
                            calls.push(child);
                        } else {
                            debug!("Skipping non-name node: kind={}", name_node.kind());
                        }
                    } else {
                        debug!("Could not get call name node for callable node");
                    }
                }
            }
        }
        
        debug!("Found {} total calls in method body", calls.len());
        calls
    }

    fn get_function_call_query(&self) -> &'static str {
        r#"
            ; Function calls (including built-ins)
            (function_call_expression
                function: (_) @func_name)

            ; Method calls with explicit $this
            (member_call_expression
                object: (_)  ; Match any object expression
                name: (name) @func_name)

            ; Static method calls
            (scoped_call_expression
                scope: (_)   ; Match any scope
                name: (name) @func_name)

            ; Object instantiation
            (object_creation_expression
                (name) @func_name)
            (object_creation_expression
                (qualified_name) @func_name)

            ; Built-in function calls
            (function_call_expression
                function: (name) @func_name)
        "#
    }

    fn get_function_definition_query(&self) -> &'static str {
        r#"
            ; Regular functions
            (function_definition
                name: (name) @func_name)

            ; Class methods (including static)
            (method_declaration
                name: (name) @func_name)
        "#
    }

    fn get_enclosing_function_pattern(&self) -> &'static str {
        "(function_definition | method_declaration | class_declaration) @cap"
    }

    fn determine_symbol_kind(&self, node_type: &str, node_text: &str) -> SymbolKind {
        match node_type {
            "class_declaration" => SymbolKind::CLASS,
            "method_declaration" => SymbolKind::METHOD,
            _ => SymbolKind::FUNCTION,
        }
    }

    fn configure_parser(&self, parser: &mut tree_sitter::Parser) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        parser.set_language(&tree_sitter_php::LANGUAGE_PHP.into())?;
        Ok(())
    }

    fn is_package_root(&self, dir: &std::path::Path) -> bool {
        dir.join("composer.json").exists() || dir.join("main.php").exists()
    }

    fn is_identifier_node(&self, node_type: &str) -> bool {
        node_type == "name"
    }

    fn is_callable_type(&self, node_type: &str) -> bool {
        matches!(node_type, 
            // Definitions
            "function_definition" |
            "method_declaration" |
            // Calls
            "function_call_expression" |
            "member_call_expression" |
            "scoped_call_expression" |
            "object_creation_expression"
        )
    }

    fn is_definition(&self, node_type: &str) -> bool {
        matches!(node_type,
            "function_definition" |
            "method_declaration" |
            "class_declaration"  // included because it can contain method definitions
        )
    }
}