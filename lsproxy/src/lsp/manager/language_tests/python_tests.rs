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
            name: String::from("plot_path"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("main.py"),
                position: Position {
                    line: 6,
                    character: 4,
                },
            },
            range: FileRange {
                path: String::from("main.py"),
                start: Position {
                    line: 5,
                    character: 0,
                },
                end: Position {
                    line: 12,
                    character: 14,
                },
            },
        },
        Symbol {
            name: String::from("main"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("main.py"),
                position: Position {
                    line: 14,
                    character: 4,
                },
            },
            range: FileRange {
                path: String::from("main.py"),
                start: Position {
                    line: 14,
                    character: 0,
                },
                end: Position {
                    line: 19,
                    character: 28,
                },
            },
        },
        Symbol {
            name: String::from("graph"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("main.py"),
                position: Position {
                    line: 15,
                    character: 4,
                },
            },
            range: FileRange {
                path: String::from("main.py"),
                start: Position {
                    line: 15,
                    character: 0,
                },
                end: Position {
                    line: 15,
                    character: 24,
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
            name: String::from("GraphBase"),
            kind: String::from("class"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 4,
                    character: 6,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 4,
                    character: 0,
                },
                end: Position {
                    line: 5,
                    character: 8,
                },
            },
        },
        Symbol {
            name: String::from("CostStrategy"),
            kind: String::from("class"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 7,
                    character: 6,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 7,
                    character: 0,
                },
                end: Position {
                    line: 10,
                    character: 25,
                },
            },
        },
        Symbol {
            name: String::from("BARRIER"),
            kind: String::from("variable"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 8,
                    character: 4,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 8,
                    character: 0,
                },
                end: Position {
                    line: 8,
                    character: 23,
                },
            },
        },
        Symbol {
            name: String::from("DISTANCE"),
            kind: String::from("variable"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 9,
                    character: 4,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
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
        Symbol {
            name: String::from("COMBINED"),
            kind: String::from("variable"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 10,
                    character: 4,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 10,
                    character: 0,
                },
                end: Position {
                    line: 10,
                    character: 25,
                },
            },
        },
        Symbol {
            name: String::from("AStarGraph"),
            kind: String::from("class"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 12,
                    character: 6,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 12,
                    character: 0,
                },
                end: Position {
                    line: 88,
                    character: 16,
                },
            },
        },
        Symbol {
            name: String::from("__init__"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 13,
                    character: 8,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 13,
                    character: 0,
                },
                end: Position {
                    line: 20,
                    character: 10,
                },
            },
        },
        Symbol {
            name: String::from("barriers"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 23,
                    character: 8,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 22,
                    character: 0,
                },
                end: Position {
                    line: 24,
                    character: 29,
                },
            },
        },
        Symbol {
            name: String::from("_barrier_cost"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 26,
                    character: 8,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 26,
                    character: 0,
                },
                end: Position {
                    line: 31,
                    character: 16,
                },
            },
        },
        Symbol {
            name: String::from("_distance_cost"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 33,
                    character: 8,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 33,
                    character: 0,
                },
                end: Position {
                    line: 35,
                    character: 50,
                },
            },
        },
        Symbol {
            name: String::from("_combined_cost"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 37,
                    character: 8,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 37,
                    character: 0,
                },
                end: Position {
                    line: 41,
                    character: 43,
                },
            },
        },
        Symbol {
            name: String::from("barrier_cost"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 39,
                    character: 8,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 39,
                    character: 0,
                },
                end: Position {
                    line: 39,
                    character: 47,
                },
            },
        },
        Symbol {
            name: String::from("distance_cost"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 40,
                    character: 8,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 40,
                    character: 0,
                },
                end: Position {
                    line: 40,
                    character: 49,
                },
            },
        },
        Symbol {
            name: String::from("move_cost"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 43,
                    character: 8,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 43,
                    character: 0,
                },
                end: Position {
                    line: 65,
                    character: 34,
                },
            },
        },
        Symbol {
            name: String::from("cost_function"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 57,
                    character: 12,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 57,
                    character: 0,
                },
                end: Position {
                    line: 57,
                    character: 46,
                },
            },
        },
        Symbol {
            name: String::from("cost_function"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 59,
                    character: 12,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 59,
                    character: 0,
                },
                end: Position {
                    line: 59,
                    character: 47,
                },
            },
        },
        Symbol {
            name: String::from("cost_function"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 61,
                    character: 12,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 61,
                    character: 0,
                },
                end: Position {
                    line: 61,
                    character: 47,
                },
            },
        },
        Symbol {
            name: String::from("heuristic"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 68,
                    character: 8,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 67,
                    character: 0,
                },
                end: Position {
                    line: 73,
                    character: 57,
                },
            },
        },
        Symbol {
            name: String::from("D"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 69,
                    character: 8,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 69,
                    character: 0,
                },
                end: Position {
                    line: 69,
                    character: 13,
                },
            },
        },
        Symbol {
            name: String::from("D2"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 70,
                    character: 8,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 70,
                    character: 0,
                },
                end: Position {
                    line: 70,
                    character: 14,
                },
            },
        },
        Symbol {
            name: String::from("dx"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 71,
                    character: 8,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 71,
                    character: 0,
                },
                end: Position {
                    line: 71,
                    character: 36,
                },
            },
        },
        Symbol {
            name: String::from("dy"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 72,
                    character: 8,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 72,
                    character: 0,
                },
                end: Position {
                    line: 72,
                    character: 36,
                },
            },
        },
        Symbol {
            name: String::from("get_vertex_neighbours"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 76,
                    character: 8,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 75,
                    character: 0,
                },
                end: Position {
                    line: 88,
                    character: 16,
                },
            },
        },
        Symbol {
            name: String::from("n"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 77,
                    character: 8,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 77,
                    character: 0,
                },
                end: Position {
                    line: 77,
                    character: 14,
                },
            },
        },
        Symbol {
            name: String::from("x2"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 82,
                    character: 12,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 82,
                    character: 0,
                },
                end: Position {
                    line: 82,
                    character: 28,
                },
            },
        },
        Symbol {
            name: String::from("y2"),
            kind: String::from("local-variable"),
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 83,
                    character: 12,
                },
            },
            range: FileRange {
                path: String::from("graph.py"),
                start: Position {
                    line: 83,
                    character: 0,
                },
                end: Position {
                    line: 83,
                    character: 28,
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
                line: 12,
                character: 6,
            },
        )
        .await?;

    let expected = vec![
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/python/graph.py").unwrap(),
            range: Range {
                start: lsp_types::Position {
                    line: 12,
                    character: 6,
                },
                end: lsp_types::Position {
                    line: 12,
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
                    line: 6,
                    character: 27,
                },
                end: lsp_types::Position {
                    line: 6,
                    character: 37,
                },
            },
        },
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/python/main.py").unwrap(),
            range: Range {
                start: lsp_types::Position {
                    line: 15,
                    character: 12,
                },
                end: lsp_types::Position {
                    line: 15,
                    character: 22,
                },
            },
        },
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/python/search.py").unwrap(),
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
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/python/search.py").unwrap(),
            range: Range {
                start: lsp_types::Position {
                    line: 5,
                    character: 41,
                },
                end: lsp_types::Position {
                    line: 5,
                    character: 51,
                },
            },
        },
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/python/search.py").unwrap(),
            range: Range {
                start: lsp_types::Position {
                    line: 16,
                    character: 37,
                },
                end: lsp_types::Position {
                    line: 16,
                    character: 47,
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
                    line: 12,
                    character: 6,
                },
                end: lsp_types::Position {
                    line: 12,
                    character: 16,
                },
            },
        }]
    );

    Ok(())
}
