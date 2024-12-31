use super::*;

#[tokio::test]
async fn test_php_file_symbols() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&php_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;
    let file_path = "AStar.php";
    let file_symbols = manager.definitions_in_file_ast_grep(file_path).await?;
    let mut symbol_response: SymbolResponse =
        file_symbols.into_iter().map(|s| Symbol::from(s)).collect();

    let mut expected = vec![
        Symbol {
            name: String::from("AStar"),
            kind: String::from("class"),
            identifier_position: FilePosition {
                path: String::from("AStar.php"),
                position: Position {
                    line: 6,
                    character: 6,
                },
            },
            range: FileRange {
                path: String::from("AStar.php"),
                start: Position {
                    line: 6,
                    character: 0,
                },
                end: Position {
                    line: 94,
                    character: 1,
                },
            },
        },
        Symbol {
            name: String::from("__construct"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("AStar.php"),
                position: Position {
                    line: 18,
                    character: 20,
                },
            },
            range: FileRange {
                path: String::from("AStar.php"),
                start: Position {
                    line: 18,
                    character: 0,
                },
                end: Position {
                    line: 24,
                    character: 5,
                },
            },
        },
        Symbol {
            name: String::from("addNeighborsToOpenList"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("AStar.php"),
                position: Position {
                    line: 50,
                    character: 21,
                },
            },
            range: FileRange {
                path: String::from("AStar.php"),
                start: Position {
                    line: 50,
                    character: 0,
                },
                end: Position {
                    line: 80,
                    character: 5,
                },
            },
        },
        Symbol {
            name: String::from("closed"),
            kind: String::from("property"),
            identifier_position: FilePosition {
                path: String::from("AStar.php"),
                position: Position {
                    line: 8,
                    character: 19,
                },
            },
            range: FileRange {
                path: String::from("AStar.php"),
                start: Position {
                    line: 8,
                    character: 0,
                },
                end: Position {
                    line: 8,
                    character: 31,
                },
            },
        },
        Symbol {
            name: String::from("diag"),
            kind: String::from("property"),
            identifier_position: FilePosition {
                path: String::from("AStar.php"),
                position: Position {
                    line: 16,
                    character: 18,
                },
            },
            range: FileRange {
                path: String::from("AStar.php"),
                start: Position {
                    line: 16,
                    character: 0,
                },
                end: Position {
                    line: 16,
                    character: 23,
                },
            },
        },
        Symbol {
            name: String::from("distance"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("AStar.php"),
                position: Position {
                    line: 82,
                    character: 21,
                },
            },
            range: FileRange {
                path: String::from("AStar.php"),
                start: Position {
                    line: 82,
                    character: 0,
                },
                end: Position {
                    line: 84,
                    character: 5,
                },
            },
        },
        Symbol {
            name: String::from("findNeighborInList"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("AStar.php"),
                position: Position {
                    line: 86,
                    character: 21,
                },
            },
            range: FileRange {
                path: String::from("AStar.php"),
                start: Position {
                    line: 86,
                    character: 0,
                },
                end: Position {
                    line: 93,
                    character: 5,
                },
            },
        },
        Symbol {
            name: String::from("findPathTo"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("AStar.php"),
                position: Position {
                    line: 26,
                    character: 20,
                },
            },
            range: FileRange {
                path: String::from("AStar.php"),
                start: Position {
                    line: 26,
                    character: 0,
                },
                end: Position {
                    line: 48,
                    character: 5,
                },
            },
        },
        Symbol {
            name: String::from("maze"),
            kind: String::from("property"),
            identifier_position: FilePosition {
                path: String::from("AStar.php"),
                position: Position {
                    line: 10,
                    character: 19,
                },
            },
            range: FileRange {
                path: String::from("AStar.php"),
                start: Position {
                    line: 10,
                    character: 0,
                },
                end: Position {
                    line: 10,
                    character: 24,
                },
            },
        },
        Symbol {
            name: String::from("now"),
            kind: String::from("property"),
            identifier_position: FilePosition {
                path: String::from("AStar.php"),
                position: Position {
                    line: 11,
                    character: 18,
                },
            },
            range: FileRange {
                path: String::from("AStar.php"),
                start: Position {
                    line: 11,
                    character: 0,
                },
                end: Position {
                    line: 11,
                    character: 22,
                },
            },
        },
        Symbol {
            name: String::from("open"),
            kind: String::from("property"),
            identifier_position: FilePosition {
                path: String::from("AStar.php"),
                position: Position {
                    line: 7,
                    character: 19,
                },
            },
            range: FileRange {
                path: String::from("AStar.php"),
                start: Position {
                    line: 7,
                    character: 0,
                },
                end: Position {
                    line: 7,
                    character: 29,
                },
            },
        },
        Symbol {
            name: String::from("path"),
            kind: String::from("property"),
            identifier_position: FilePosition {
                path: String::from("AStar.php"),
                position: Position {
                    line: 9,
                    character: 19,
                },
            },
            range: FileRange {
                path: String::from("AStar.php"),
                start: Position {
                    line: 9,
                    character: 0,
                },
                end: Position {
                    line: 9,
                    character: 29,
                },
            },
        },
        Symbol {
            name: String::from("xend"),
            kind: String::from("property"),
            identifier_position: FilePosition {
                path: String::from("AStar.php"),
                position: Position {
                    line: 14,
                    character: 17,
                },
            },
            range: FileRange {
                path: String::from("AStar.php"),
                start: Position {
                    line: 14,
                    character: 0,
                },
                end: Position {
                    line: 14,
                    character: 22,
                },
            },
        },
        Symbol {
            name: String::from("xstart"),
            kind: String::from("property"),
            identifier_position: FilePosition {
                path: String::from("AStar.php"),
                position: Position {
                    line: 12,
                    character: 17,
                },
            },
            range: FileRange {
                path: String::from("AStar.php"),
                start: Position {
                    line: 12,
                    character: 0,
                },
                end: Position {
                    line: 12,
                    character: 24,
                },
            },
        },
        Symbol {
            name: String::from("yend"),
            kind: String::from("property"),
            identifier_position: FilePosition {
                path: String::from("AStar.php"),
                position: Position {
                    line: 15,
                    character: 17,
                },
            },
            range: FileRange {
                path: String::from("AStar.php"),
                start: Position {
                    line: 15,
                    character: 0,
                },
                end: Position {
                    line: 15,
                    character: 22,
                },
            },
        },
        Symbol {
            name: String::from("ystart"),
            kind: String::from("property"),
            identifier_position: FilePosition {
                path: String::from("AStar.php"),
                position: Position {
                    line: 13,
                    character: 17,
                },
            },
            range: FileRange {
                path: String::from("AStar.php"),
                start: Position {
                    line: 13,
                    character: 0,
                },
                end: Position {
                    line: 13,
                    character: 24,
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
async fn test_php_references() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&php_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;
    let file_path = "Node.php";
    let references = manager
        .find_references(
            file_path,
            lsp_types::Position {
                line: 3,
                character: 6,
            },
        )
        .await?;

    let expected = vec![
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/php/AStar.php").unwrap(),
            range: Range {
                start: lsp_types::Position {
                    line: 11,
                    character: 12,
                },
                end: lsp_types::Position {
                    line: 11,
                    character: 16,
                },
            },
        },
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/php/AStar.php").unwrap(),
            range: Range {
                start: lsp_types::Position {
                    line: 23,
                    character: 25,
                },
                end: lsp_types::Position {
                    line: 23,
                    character: 29,
                },
            },
        },
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/php/AStar.php").unwrap(),
            range: Range {
                start: lsp_types::Position {
                    line: 57,
                    character: 28,
                },
                end: lsp_types::Position {
                    line: 57,
                    character: 32,
                },
            },
        },
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/php/AStar.php").unwrap(),
            range: Range {
                start: lsp_types::Position {
                    line: 86,
                    character: 53,
                },
                end: lsp_types::Position {
                    line: 86,
                    character: 57,
                },
            },
        },
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/php/Node.php").unwrap(),
            range: Range {
                start: lsp_types::Position {
                    line: 3,
                    character: 0,
                },
                end: lsp_types::Position {
                    line: 23,
                    character: 1,
                },
            },
        },
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/php/Node.php").unwrap(),
            range: Range {
                start: lsp_types::Position {
                    line: 4,
                    character: 12,
                },
                end: lsp_types::Position {
                    line: 4,
                    character: 16,
                },
            },
        },
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/php/Node.php").unwrap(),
            range: Range {
                start: lsp_types::Position {
                    line: 10,
                    character: 33,
                },
                end: lsp_types::Position {
                    line: 10,
                    character: 37,
                },
            },
        },
    ];
    assert_eq!(references, expected);
    Ok(())
}

#[tokio::test]
async fn test_php_definition() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&php_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;

    let definition_response = manager
        .find_definition(
            "main.php",
            lsp_types::Position {
                line: 20,
                character: 13,
            },
        )
        .await?;

    let definitions = match definition_response {
        GotoDefinitionResponse::Scalar(location) => vec![location],
        GotoDefinitionResponse::Array(locations) => locations,
        GotoDefinitionResponse::Link(_links) => Vec::new(),
    };
    let expected = vec![Location {
        uri: Url::parse("file:///mnt/lsproxy_root/sample_project/php/AStar.php").unwrap(),
        range: Range {
            start: lsp_types::Position {
                line: 6,
                character: 0,
            },
            end: lsp_types::Position {
                line: 94,
                character: 1,
            },
        },
    }];

    assert_eq!(definitions, expected);
    Ok(())
}
