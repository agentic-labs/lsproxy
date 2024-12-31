use super::*;

#[tokio::test]
async fn test_start_manager() -> Result<(), Box<dyn std::error::Error>> {
    TestContext::setup(&python_sample_path(), true).await?;
    Ok(())
}

#[tokio::test]
async fn test_workspace_files() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&python_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;

    let mut result = manager.list_files().await?;
    let mut expected = vec!["graph.py", "main.py", "search.py", "__init__.py"];

    assert_eq!(result.sort(), expected.sort());
    Ok(())
}

#[tokio::test]
async fn test_file_symbols() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&python_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;

    let file_path = "main.py";
    let file_symbols = manager.definitions_in_file_ast_grep(file_path).await?;

    let symbol_response: SymbolResponse =
        file_symbols.into_iter().map(|s| Symbol::from(s)).collect();

    let expected = vec![
        Symbol {
            name: String::from("graph"),
            kind: String::from("variable"),
            identifier_position: FilePosition {
                path: String::from("main.py"),
                position: Position {
                    line: 5,
                    character: 0,
                },
            },
            range: FileRange {
                path: String::from("main.py"),
                start: Position {
                    line: 5,
                    character: 0,
                },
                end: Position {
                    line: 5,
                    character: 20,
                },
            },
        },
        Symbol {
            name: String::from("result"),
            kind: String::from("variable"),
            identifier_position: FilePosition {
                path: String::from("main.py"),
                position: Position {
                    line: 6,
                    character: 0,
                },
            },
            range: FileRange {
                path: String::from("main.py"),
                start: Position {
                    line: 6,
                    character: 0,
                },
                end: Position {
                    line: 6,
                    character: 51,
                },
            },
        },
        Symbol {
            name: String::from("cost"),
            kind: String::from("variable"),
            identifier_position: FilePosition {
                path: String::from("main.py"),
                position: Position {
                    line: 6,
                    character: 8,
                },
            },
            range: FileRange {
                path: String::from("main.py"),
                start: Position {
                    line: 6,
                    character: 0,
                },
                end: Position {
                    line: 6,
                    character: 51,
                },
            },
        },
    ];
    assert_eq!(symbol_response, expected);
    Ok(())
}

#[tokio::test]
async fn test_file_symbols_decorators() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&python_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;

    let file_path = "graph.py";
    let file_symbols = manager.definitions_in_file_ast_grep(file_path).await?;

    let symbol_response: SymbolResponse =
        file_symbols.into_iter().map(|s| Symbol::from(s)).collect();

    let expected = vec![
        Symbol {
            name: String::from("AStarGraph"),
            kind: String::from("class"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 1,
                    character: 6,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 1,
                    character: 0,
                },
                end: Position {
                    line: 60,
                    character: 40,
                },
            },
        },
        Symbol {
            name: String::from("__init__"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 4,
                    character: 8,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 4,
                    character: 0,
                },
                end: Position {
                    line: 21,
                    character: 9,
                },
            },
        },
        Symbol {
            name: String::from("barriers"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 24,
                    character: 8,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 23,
                    character: 0,
                },
                end: Position {
                    line: 25,
                    character: 28,
                },
            },
        },
        Symbol {
            name: String::from("heuristic"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 27,
                    character: 8,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 27,
                    character: 0,
                },
                end: Position {
                    line: 34,
                    character: 57,
                },
            },
        },
        Symbol {
            name: String::from("get_vertex_neighbours"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 36,
                    character: 8,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 36,
                    character: 0,
                },
                end: Position {
                    line: 54,
                    character: 16,
                },
            },
        },
        Symbol {
            name: String::from("move_cost"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 56,
                    character: 8,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 56,
                    character: 0,
                },
                end: Position {
                    line: 60,
                    character: 40,
                },
            },
        },
    ];
    assert_eq!(symbol_response, expected);
    Ok(())
}

#[tokio::test]
async fn test_references() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&python_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;
    let file_path = "graph.py";

    let references = manager
        .find_references(
            file_path,
            lsp_types::Position {
                line: 1,
                character: 6,
            },
        )
        .await?;

    let expected = vec![
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/python/graph.py").unwrap(),
            range: Range {
                start: lsp_types::Position {
                    line: 1,
                    character: 6,
                },
                end: lsp_types::Position {
                    line: 1,
                    character: 16,
                },
            },
        },
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/python/main.py").unwrap(),
            range: Range {
                start: lsp_types::Position {
                    line: 1,
                    character: 18,
                },
                end: lsp_types::Position {
                    line: 1,
                    character: 28,
                },
            },
        },
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/python/main.py").unwrap(),
            range: Range {
                start: lsp_types::Position {
                    line: 5,
                    character: 8,
                },
                end: lsp_types::Position {
                    line: 5,
                    character: 18,
                },
            },
        },
    ];
    assert_eq!(references, expected);

    Ok(())
}

#[tokio::test]
async fn test_definition() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&python_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;
    let def_response = manager
        .find_definition(
            "main.py",
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
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/python/graph.py").unwrap(),
            range: Range {
                start: lsp_types::Position {
                    line: 1,
                    character: 6,
                },
                end: lsp_types::Position {
                    line: 1,
                    character: 16,
                },
            },
        }]
    );

    Ok(())
}
