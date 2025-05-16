use super::*;
use crate::api_types::{Position, Range};

#[tokio::test]
async fn test_start_manager() -> Result<(), Box<dyn std::error::Error>> {
    TestContext::setup(&js_sample_path(), true).await?;
    Ok(())
}

#[tokio::test]
async fn test_workspace_files() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&js_sample_path(), true).await?;

    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;
    let files = manager.list_files().await?;

    assert_eq!(files, vec!["astar_search.js", "functions.js", "methods.js"]);
    Ok(())
}

#[tokio::test]
async fn test_references() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&js_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;

    let file_path = "astar_search.js";

    let references = manager
        .find_references(
            file_path,
            lsp_types::Position {
                line: 0,
                character: 9,
            },
        )
        .await?;

    let expected = vec![
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/js/astar_search.js")?,
            range: lsp_types::Range {
                start: lsp_types::Position {
                    line: 0,
                    character: 9,
                },
                end: lsp_types::Position {
                    line: 0,
                    character: 18,
                },
            },
        },
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/js/astar_search.js")?,
            range: lsp_types::Range {
                start: lsp_types::Position {
                    line: 10,
                    character: 21,
                },
                end: lsp_types::Position {
                    line: 10,
                    character: 30,
                },
            },
        },
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/js/astar_search.js")?,
            range: lsp_types::Range {
                start: lsp_types::Position {
                    line: 40,
                    character: 25,
                },
                end: lsp_types::Position {
                    line: 40,
                    character: 34,
                },
            },
        },
    ];
    assert_eq!(references, expected);
    Ok(())
}

#[tokio::test]
async fn test_definition() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&js_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;
    let def_response = manager
        .find_definition(
            "astar_search.js",
            lsp_types::Position {
                line: 1,
                character: 18,
            },
        )
        .await?;

    let definitions = match def_response {
        GotoDefinitionResponse::Scalar(location) => vec![location],
        GotoDefinitionResponse::Array(locations) => locations,
        GotoDefinitionResponse::Link(_links) => Vec::new(),
    };

    assert_eq!(
        definitions,
        vec![Location {
            uri: Url::parse("file:///usr/lib/node_modules/typescript/lib/lib.es5.d.ts")?,
            range: lsp_types::Range {
                start: lsp_types::Position {
                    line: 681,
                    character: 4
                },
                end: lsp_types::Position {
                    line: 681,
                    character: 7
                }
            }
        }]
    );
    Ok(())
}

#[tokio::test]
async fn test_file_symbols() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&js_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;

    let file_path = "astar_search.js";
    let file_symbols = manager.definitions_in_file_ast_grep(file_path).await?;
    // TODO: include source code and update expected
    let mut symbol_response: SymbolResponse = file_symbols.into_iter().map(Symbol::from).collect();

    let mut expected = vec![
        Symbol {
            name: String::from("manhattan"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("astar_search.js"),
                position: Position {
                    line: 0,
                    character: 9,
                },
            },
            file_range: FileRange {
                path: String::from("astar_search.js"),
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 2,
                        character: 1,
                    },
                },
            },
        },
        Symbol {
            name: String::from("aStar"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("astar_search.js"),
                position: Position {
                    line: 4,
                    character: 9,
                },
            },
            file_range: FileRange {
                path: String::from("astar_search.js"),
                range: Range {
                    start: Position {
                        line: 4,
                        character: 0,
                    },
                    end: Position {
                        line: 58,
                        character: 1,
                    },
                },
            },
        },
        Symbol {
            name: String::from("lambda"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("astar_search.js"),
                position: Position {
                    line: 17,
                    character: 16,
                },
            },
            file_range: FileRange {
                path: String::from("astar_search.js"),
                range: Range {
                    start: Position {
                        line: 17,
                        character: 0,
                    },
                    end: Position {
                        line: 26,
                        character: 9,
                    },
                },
            },
        },
        Symbol {
            name: String::from("board"),
            kind: String::from("variable"),
            identifier_position: FilePosition {
                path: String::from("astar_search.js"),
                position: Position {
                    line: 60,
                    character: 6,
                },
            },
            file_range: FileRange {
                path: String::from("astar_search.js"),
                range: Range {
                    start: Position {
                        line: 60,
                        character: 0,
                    },
                    end: Position {
                        line: 69,
                        character: 1,
                    },
                },
            },
        },
    ];

    // sort symbols by name
    symbol_response.sort_by_key(|s| s.name.clone());
    expected.sort_by_key(|s| s.name.clone());
    assert_eq!(symbol_response, expected);
    Ok(())
}

#[tokio::test]
async fn test_file_symbols_functions_js() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&js_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;

    let file_path = "functions.js";
    let file_symbols = manager.definitions_in_file_ast_grep(file_path).await?;
    let mut symbol_response: SymbolResponse = file_symbols.into_iter().map(Symbol::from).collect();

    let mut expected = vec![
        Symbol {
            name: "objWithFuncExpr".to_string(),
            kind: "variable".to_string(),
            identifier_position: FilePosition {
                path: file_path.to_string(),
                position: Position {
                    line: 1,
                    character: 6,
                },
            },
            file_range: FileRange {
                path: file_path.to_string(),
                range: Range {
                    start: Position {
                        line: 1,
                        character: 0,
                    },
                    end: Position {
                        line: 5,
                        character: 1,
                    },
                },
            },
        },
        Symbol {
            name: "propFuncExpr".to_string(),
            kind: "function".to_string(),
            identifier_position: FilePosition {
                path: file_path.to_string(),
                position: Position {
                    line: 2,
                    character: 2,
                },
            },
            file_range: FileRange {
                path: file_path.to_string(),
                range: Range {
                    start: Position {
                        line: 2,
                        character: 0,
                    },
                    end: Position {
                        line: 4,
                        character: 3,
                    },
                },
            },
        },
        Symbol {
            name: "objWithArrowFunc".to_string(),
            kind: "variable".to_string(),
            identifier_position: FilePosition {
                path: file_path.to_string(),
                position: Position {
                    line: 8,
                    character: 6,
                },
            },
            file_range: FileRange {
                path: file_path.to_string(),
                range: Range {
                    start: Position {
                        line: 8,
                        character: 0,
                    },
                    end: Position {
                        line: 10,
                        character: 1,
                    },
                },
            },
        },
        Symbol {
            name: "propArrowFunc".to_string(),
            kind: "function".to_string(),
            identifier_position: FilePosition {
                path: file_path.to_string(),
                position: Position {
                    line: 9,
                    character: 2,
                },
            },
            file_range: FileRange {
                path: file_path.to_string(),
                range: Range {
                    start: Position {
                        line: 9,
                        character: 0,
                    },
                    end: Position {
                        line: 9,
                        character: 25,
                    },
                },
            },
        },
        Symbol {
            name: "topLevelStandardFunction".to_string(),
            kind: "function".to_string(),
            identifier_position: FilePosition {
                path: file_path.to_string(),
                position: Position {
                    line: 13,
                    character: 9,
                },
            },
            file_range: FileRange {
                path: file_path.to_string(),
                range: Range {
                    start: Position {
                        line: 13,
                        character: 0,
                    },
                    end: Position {
                        line: 13,
                        character: 38,
                    },
                },
            },
        },
        Symbol {
            name: "topLevelArrowConst".to_string(),
            kind: "function".to_string(),
            identifier_position: FilePosition {
                path: file_path.to_string(),
                position: Position {
                    line: 16,
                    character: 6,
                },
            },
            file_range: FileRange {
                // Assuming range covers the 'const ...;' declaration
                path: file_path.to_string(),
                range: Range {
                    start: Position {
                        line: 16,
                        character: 0,
                    },
                    end: Position {
                        line: 16,
                        character: 35,
                    },
                },
            },
        },
        Symbol {
            name: "namedInnerFuncExpr".to_string(),
            kind: "function".to_string(),
            identifier_position: FilePosition {
                path: file_path.to_string(),
                position: Position {
                    line: 19,
                    character: 39,
                },
            },
            file_range: FileRange {
                path: file_path.to_string(),
                range: Range {
                    start: Position {
                        line: 19,
                        character: 0,
                    },
                    end: Position {
                        line: 19,
                        character: 62,
                    },
                },
            },
        },
        Symbol {
            name: "topLevelFuncExprConst".to_string(),
            kind: "variable".to_string(),
            identifier_position: FilePosition {
                path: file_path.to_string(),
                position: Position {
                    line: 19,
                    character: 6,
                },
            },
            file_range: FileRange {
                path: file_path.to_string(),
                range: Range {
                    start: Position {
                        line: 19,
                        character: 0,
                    },
                    end: Position {
                        line: 19,
                        character: 62,
                    },
                },
            },
        },
        Symbol {
            name: "assignedArrowLet".to_string(),
            kind: "variable".to_string(),
            identifier_position: FilePosition {
                path: file_path.to_string(),
                position: Position {
                    line: 22,
                    character: 4,
                },
            },
            file_range: FileRange {
                // Range of the assignment expression
                path: file_path.to_string(),
                range: Range {
                    start: Position {
                        line: 22,
                        character: 0,
                    },
                    end: Position {
                        line: 22,
                        character: 20,
                    },
                },
            },
        },
        Symbol {
            name: "assignedArrowLet".to_string(),
            kind: "function".to_string(),
            identifier_position: FilePosition {
                path: file_path.to_string(),
                position: Position {
                    line: 23,
                    character: 0,
                },
            },
            file_range: FileRange {
                // Range of the assignment expression
                path: file_path.to_string(),
                range: Range {
                    start: Position {
                        line: 23,
                        character: 0,
                    },
                    end: Position {
                        line: 23,
                        character: 27,
                    },
                },
            },
        },
    ];

    symbol_response.sort_by_key(|s| s.name.clone());
    expected.sort_by_key(|s| s.name.clone());

    if symbol_response != expected {
        eprintln!("Actual symbols count: {}", symbol_response.len());
        eprintln!("Expected symbols count: {}", expected.len());
        for i in 0..std::cmp::max(symbol_response.len(), expected.len()) {
            eprintln!("--- Symbol {} ---", i);
            if i < symbol_response.len() {
                eprintln!("Actual:   {:?}", symbol_response[i]);
            } else {
                eprintln!("Actual:   None");
            }
            if i < expected.len() {
                eprintln!("Expected: {:?}", expected[i]);
            } else {
                eprintln!("Expected: None");
            }
        }
    }

    assert_eq!(
        symbol_response, expected,
        "Symbols from functions.js do not match expected symbols."
    );
    Ok(())
}

#[tokio::test]
async fn test_file_symbols_methods_js() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&js_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;

    let file_path = "methods.js";
    let file_symbols = manager.definitions_in_file_ast_grep(file_path).await?;
    let mut symbol_response: SymbolResponse = file_symbols.into_iter().map(Symbol::from).collect();

    let mut expected = vec![
        Symbol {
            name: "MyClassExample".to_string(),
            kind: "class".to_string(),
            identifier_position: FilePosition {
                path: file_path.to_string(),
                position: Position {
                    line: 1,
                    character: 6,
                },
            },
            file_range: FileRange {
                path: file_path.to_string(),
                range: Range {
                    start: Position {
                        line: 1,
                        character: 0,
                    },
                    end: Position {
                        line: 13,
                        character: 1,
                    },
                },
            },
        },
        Symbol {
            name: "classMethodRegular".to_string(),
            kind: "method".to_string(),
            identifier_position: FilePosition {
                path: file_path.to_string(),
                position: Position {
                    line: 2,
                    character: 2,
                },
            },
            file_range: FileRange {
                path: file_path.to_string(),
                range: Range {
                    start: Position {
                        line: 2,
                        character: 0,
                    },
                    end: Position {
                        line: 2,
                        character: 25,
                    },
                },
            },
        },
        Symbol {
            name: "staticClassMethod".to_string(),
            kind: "method".to_string(),
            identifier_position: FilePosition {
                path: file_path.to_string(),
                position: Position {
                    line: 4,
                    character: 9,
                },
            },
            file_range: FileRange {
                path: file_path.to_string(),
                range: Range {
                    start: Position {
                        line: 4,
                        character: 0,
                    },
                    end: Position {
                        line: 4,
                        character: 31,
                    },
                },
            },
        },
        Symbol {
            name: "getterMethod".to_string(),
            kind: "method".to_string(),
            identifier_position: FilePosition {
                path: file_path.to_string(),
                position: Position {
                    line: 6,
                    character: 6,
                },
            },
            file_range: FileRange {
                path: file_path.to_string(),
                range: Range {
                    start: Position {
                        line: 6,
                        character: 0,
                    },
                    end: Position {
                        line: 8,
                        character: 3,
                    },
                },
            },
        },
        Symbol {
            name: "setterMethod".to_string(),
            kind: "method".to_string(),
            identifier_position: FilePosition {
                path: file_path.to_string(),
                position: Position {
                    line: 10,
                    character: 6,
                },
            },
            file_range: FileRange {
                path: file_path.to_string(),
                range: Range {
                    start: Position {
                        line: 10,
                        character: 0,
                    },
                    end: Position {
                        line: 12,
                        character: 3,
                    },
                },
            },
        },
        Symbol {
            name: "objWithShorthand".to_string(),
            kind: "variable".to_string(),
            identifier_position: FilePosition {
                path: file_path.to_string(),
                position: Position {
                    line: 16,
                    character: 6,
                },
            },
            file_range: FileRange {
                path: file_path.to_string(),
                range: Range {
                    start: Position {
                        line: 16,
                        character: 0,
                    },
                    end: Position {
                        line: 22,
                        character: 1,
                    },
                },
            },
        },
        Symbol {
            name: "shorthandObjMethod".to_string(),
            kind: "method".to_string(),
            identifier_position: FilePosition {
                path: file_path.to_string(),
                position: Position {
                    line: 17,
                    character: 2,
                },
            },
            file_range: FileRange {
                path: file_path.to_string(),
                range: Range {
                    start: Position {
                        line: 17,
                        character: 0,
                    },
                    end: Position {
                        line: 17,
                        character: 25,
                    },
                },
            },
        },
        Symbol {
            name: "generatorShorthandMethod".to_string(),
            kind: "method".to_string(),
            identifier_position: FilePosition {
                path: file_path.to_string(),
                position: Position {
                    line: 19,
                    character: 3,
                },
            },
            file_range: FileRange {
                path: file_path.to_string(),
                range: Range {
                    start: Position {
                        line: 19,
                        character: 0,
                    },
                    end: Position {
                        line: 19,
                        character: 32,
                    },
                },
            },
        },
        Symbol {
            name: "asyncShorthandMethod".to_string(),
            kind: "method".to_string(),
            identifier_position: FilePosition {
                path: file_path.to_string(),
                position: Position {
                    line: 21,
                    character: 8,
                },
            },
            file_range: FileRange {
                path: file_path.to_string(),
                range: Range {
                    start: Position {
                        line: 21,
                        character: 0,
                    },
                    end: Position {
                        line: 21,
                        character: 33,
                    },
                },
            },
        },
    ];

    symbol_response.sort_by_key(|s| s.name.clone());
    expected.sort_by_key(|s| s.name.clone());

    if symbol_response != expected {
        eprintln!("Actual symbols count: {}", symbol_response.len());
        eprintln!("Expected symbols count: {}", expected.len());
        for i in 0..std::cmp::max(symbol_response.len(), expected.len()) {
            eprintln!("--- Symbol {} ---", i);
            if i < symbol_response.len() {
                eprintln!("Actual:   {:?}", symbol_response[i]);
            } else {
                eprintln!("Actual:   None");
            }
            if i < expected.len() {
                eprintln!("Expected: {:?}", expected[i]);
            } else {
                eprintln!("Expected: None");
            }
        }
    }

    assert_eq!(
        symbol_response, expected,
        "Symbols from methods.js do not match expected symbols."
    );
    Ok(())
}
