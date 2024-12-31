use super::*;

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

    assert_eq!(files, vec!["astar_search.js"]);
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
            range: Range {
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
            range: Range {
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
            range: Range {
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
            range: Range {
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
    let mut symbol_response: SymbolResponse =
        file_symbols.into_iter().map(|s| Symbol::from(s)).collect();

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
            range: FileRange {
                path: String::from("astar_search.js"),
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
            range: FileRange {
                path: String::from("astar_search.js"),
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
            range: FileRange {
                path: String::from("astar_search.js"),
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
            range: FileRange {
                path: String::from("astar_search.js"),
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
    ];

    // sort symbols by name
    symbol_response.sort_by_key(|s| s.name.clone());
    expected.sort_by_key(|s| s.name.clone());
    assert_eq!(symbol_response, expected);
    Ok(())
}
