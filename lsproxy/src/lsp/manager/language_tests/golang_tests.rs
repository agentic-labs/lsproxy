use super::*;

#[tokio::test]
async fn test_file_symbols() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&go_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;
    let file_path = "golang_astar/search.go";
    let file_symbols = manager.definitions_in_file_ast_grep(file_path).await?;
    let mut symbol_response: SymbolResponse =
        file_symbols.into_iter().map(|s| Symbol::from(s)).collect();

    let mut expected = vec![
        Symbol {
            name: "FindPath".to_string(),
            kind: "function".to_string(),
            identifier_position: FilePosition {
                path: file_path.to_string(),
                position: Position {
                    line: 57,
                    character: 5,
                },
            },
            range: FileRange {
                path: file_path.to_string(),
                start: Position {
                    line: 57,
                    character: 0,
                },
                end: Position {
                    line: 122,
                    character: 1,
                },
            },
        },
        Symbol {
            name: "Heuristic".to_string(),
            kind: "function".to_string(),
            identifier_position: FilePosition {
                path: file_path.to_string(),
                position: Position {
                    line: 41,
                    character: 5,
                },
            },
            range: FileRange {
                path: file_path.to_string(),
                start: Position {
                    line: 41,
                    character: 0,
                },
                end: Position {
                    line: 54,
                    character: 1,
                },
            },
        },
        Symbol {
            name: "Len".to_string(),
            kind: "method".to_string(),
            identifier_position: FilePosition {
                path: file_path.to_string(),
                position: Position {
                    line: 17,
                    character: 18,
                },
            },
            range: FileRange {
                path: file_path.to_string(),
                start: Position {
                    line: 17,
                    character: 0,
                },
                end: Position {
                    line: 17,
                    character: 55,
                },
            },
        },
        Symbol {
            name: "Less".to_string(),
            kind: "method".to_string(),
            identifier_position: FilePosition {
                path: file_path.to_string(),
                position: Position {
                    line: 18,
                    character: 18,
                },
            },
            range: FileRange {
                path: file_path.to_string(),
                start: Position {
                    line: 18,
                    character: 0,
                },
                end: Position {
                    line: 18,
                    character: 64,
                },
            },
        },
        Symbol {
            name: "Pop".to_string(),
            kind: "method".to_string(),
            identifier_position: FilePosition {
                path: file_path.to_string(),
                position: Position {
                    line: 30,
                    character: 19,
                },
            },
            range: FileRange {
                path: file_path.to_string(),
                start: Position {
                    line: 30,
                    character: 0,
                },
                end: Position {
                    line: 38,
                    character: 1,
                },
            },
        },
        Symbol {
            name: "Push".to_string(),
            kind: "method".to_string(),
            identifier_position: FilePosition {
                path: file_path.to_string(),
                position: Position {
                    line: 24,
                    character: 19,
                },
            },
            range: FileRange {
                path: file_path.to_string(),
                start: Position {
                    line: 24,
                    character: 0,
                },
                end: Position {
                    line: 29,
                    character: 1,
                },
            },
        },
        Symbol {
            name: "Swap".to_string(),
            kind: "method".to_string(),
            identifier_position: FilePosition {
                path: file_path.to_string(),
                position: Position {
                    line: 19,
                    character: 18,
                },
            },
            range: FileRange {
                path: file_path.to_string(),
                start: Position {
                    line: 19,
                    character: 0,
                },
                end: Position {
                    line: 23,
                    character: 1,
                },
            },
        },
        Symbol {
            name: "nodeHeap".to_string(),
            kind: "type".to_string(),
            identifier_position: FilePosition {
                path: file_path.to_string(),
                position: Position {
                    line: 15,
                    character: 5,
                },
            },
            range: FileRange {
                path: file_path.to_string(),
                start: Position {
                    line: 15,
                    character: 0,
                },
                end: Position {
                    line: 15,
                    character: 27,
                },
            },
        },
        Symbol {
            name: "searchNode".to_string(),
            kind: "type".to_string(),
            identifier_position: FilePosition {
                path: file_path.to_string(),
                position: Position {
                    line: 7,
                    character: 5,
                },
            },
            range: FileRange {
                path: file_path.to_string(),
                start: Position {
                    line: 7,
                    character: 0,
                },
                end: Position {
                    line: 12,
                    character: 1,
                },
            },
        },
    ];

    symbol_response.sort_by_key(|s| s.name.clone());
    expected.sort_by_key(|s| s.name.clone());
    assert_eq!(symbol_response, expected);
    Ok(())
}

#[tokio::test]
async fn test_references() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&go_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;

    let references = manager
        .find_references(
            "golang_astar/search.go",
            lsp_types::Position {
                line: 58,
                character: 5,
            },
        )
        .await?;

    let expected = vec![
        Location {
            uri: format!("file://{}/golang_astar/search.go", go_sample_path())
                .parse()
                .unwrap(),
            range: Range {
                start: lsp_types::Position {
                    line: 58,
                    character: 1,
                },
                end: lsp_types::Position {
                    line: 58,
                    character: 8,
                },
            },
        },
        Location {
            uri: format!("file://{}/golang_astar/search.go", go_sample_path())
                .parse()
                .unwrap(),
            range: Range {
                start: lsp_types::Position {
                    line: 59,
                    character: 11,
                },
                end: lsp_types::Position {
                    line: 59,
                    character: 18,
                },
            },
        },
        Location {
            uri: format!("file://{}/golang_astar/search.go", go_sample_path())
                .parse()
                .unwrap(),
            range: Range {
                start: lsp_types::Position {
                    line: 68,
                    character: 11,
                },
                end: lsp_types::Position {
                    line: 68,
                    character: 18,
                },
            },
        },
        Location {
            uri: format!("file://{}/golang_astar/search.go", go_sample_path())
                .parse()
                .unwrap(),
            range: Range {
                start: lsp_types::Position {
                    line: 72,
                    character: 5,
                },
                end: lsp_types::Position {
                    line: 72,
                    character: 12,
                },
            },
        },
        Location {
            uri: format!("file://{}/golang_astar/search.go", go_sample_path())
                .parse()
                .unwrap(),
            range: Range {
                start: lsp_types::Position {
                    line: 73,
                    character: 22,
                },
                end: lsp_types::Position {
                    line: 73,
                    character: 29,
                },
            },
        },
        Location {
            uri: format!("file://{}/golang_astar/search.go", go_sample_path())
                .parse()
                .unwrap(),
            range: Range {
                start: lsp_types::Position {
                    line: 96,
                    character: 25,
                },
                end: lsp_types::Position {
                    line: 96,
                    character: 32,
                },
            },
        },
        Location {
            uri: format!("file://{}/golang_astar/search.go", go_sample_path())
                .parse()
                .unwrap(),
            range: Range {
                start: lsp_types::Position {
                    line: 111,
                    character: 14,
                },
                end: lsp_types::Position {
                    line: 111,
                    character: 21,
                },
            },
        },
        Location {
            uri: format!("file://{}/golang_astar/search.go", go_sample_path())
                .parse()
                .unwrap(),
            range: Range {
                start: lsp_types::Position {
                    line: 116,
                    character: 13,
                },
                end: lsp_types::Position {
                    line: 116,
                    character: 20,
                },
            },
        },
    ];

    // Sort locations before comparing to ensure consistent order
    let mut actual_locations = references;
    let mut expected_locations = expected;

    actual_locations.sort_by(|a, b| a.uri.path().cmp(&b.uri.path()));
    expected_locations.sort_by(|a, b| a.uri.path().cmp(&b.uri.path()));

    assert_eq!(actual_locations, expected_locations);
    Ok(())
}

#[tokio::test]
async fn test_definition() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&go_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;

    let def_response = manager
        .find_definition(
            "main.go",
            lsp_types::Position {
                line: 26,
                character: 33,
            },
        )
        .await?;

    let definitions = match def_response {
        GotoDefinitionResponse::Scalar(loc) => vec![loc],
        GotoDefinitionResponse::Array(locs) => locs,
        GotoDefinitionResponse::Link(links) => links
            .into_iter()
            .map(|link| Location {
                uri: link.target_uri,
                range: link.target_range,
            })
            .collect(),
    };

    let expected = vec![Location {
        uri: format!("file://{}/golang_astar/search.go", go_sample_path())
            .parse()
            .unwrap(),
        range: Range {
            start: lsp_types::Position {
                line: 57,
                character: 5,
            },
            end: lsp_types::Position {
                line: 57,
                character: 13,
            },
        },
    }];
    assert_eq!(definitions, expected);
    Ok(())
}
