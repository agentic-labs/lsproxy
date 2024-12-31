use super::*;

#[tokio::test]
async fn test_file_symbols() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&rust_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;
    let file_path = "src/map.rs";
    let file_symbols = manager.definitions_in_file_ast_grep(file_path).await?;
    let mut symbol_response: SymbolResponse =
        file_symbols.into_iter().map(|s| Symbol::from(s)).collect();

    let mut expected = vec![
        Symbol {
            name: String::from("Map"),
            kind: String::from("struct"),
            identifier_position: FilePosition {
                path: String::from("src/map.rs"),
                position: Position {
                    line: 0,
                    character: 11,
                },
            },
            range: FileRange {
                path: String::from("src/map.rs"),
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 4,
                    character: 1,
                },
            },
        },
        Symbol {
            name: String::from("Map"),
            kind: String::from("implementation"),
            identifier_position: FilePosition {
                path: String::from("src/map.rs"),
                position: Position {
                    line: 6,
                    character: 5,
                },
            },
            range: FileRange {
                path: String::from("src/map.rs"),
                start: Position {
                    line: 6,
                    character: 0,
                },
                end: Position {
                    line: 24,
                    character: 1,
                },
            },
        },
        Symbol {
            name: String::from("get"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("src/map.rs"),
                position: Position {
                    line: 21,
                    character: 11,
                },
            },
            range: FileRange {
                path: String::from("src/map.rs"),
                start: Position {
                    line: 21,
                    character: 0,
                },
                end: Position {
                    line: 23,
                    character: 5,
                },
            },
        },
        Symbol {
            name: String::from("new"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("src/map.rs"),
                position: Position {
                    line: 7,
                    character: 11,
                },
            },
            range: FileRange {
                path: String::from("src/map.rs"),
                start: Position {
                    line: 7,
                    character: 0,
                },
                end: Position {
                    line: 19,
                    character: 5,
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
async fn test_workspace_files() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&rust_sample_path(), true).await?;

    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;
    let files = manager.list_files().await?;

    assert_eq!(
        files,
        vec![
            "src/astar.rs",
            "src/main.rs",
            "src/map.rs",
            "src/node.rs",
            "src/point.rs"
        ]
    );
    Ok(())
}

#[tokio::test]
async fn test_references() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&rust_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;

    let file_path = "src/node.rs";

    sleep(Duration::from_secs(5)).await;

    let mut references = manager
        .find_references(
            file_path,
            lsp_types::Position {
                line: 3,
                character: 11,
            },
        )
        .await?;

    references.sort_by(|a, b| {
        a.uri.to_string().cmp(&b.uri.to_string()).then_with(|| {
            a.range
                .start
                .line
                .cmp(&b.range.start.line)
                .then_with(|| a.range.start.character.cmp(&b.range.start.character))
        })
    });
    let mut expected = vec![
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/rust/src/node.rs")?,
            range: Range {
                start: lsp_types::Position {
                    line: 3,
                    character: 11,
                },
                end: lsp_types::Position {
                    line: 3,
                    character: 15,
                },
            },
        },
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/rust/src/node.rs")?,
            range: Range {
                start: lsp_types::Position {
                    line: 10,
                    character: 20,
                },
                end: lsp_types::Position {
                    line: 10,
                    character: 24,
                },
            },
        },
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/rust/src/node.rs")?,
            range: Range {
                start: lsp_types::Position {
                    line: 11,
                    character: 34,
                },
                end: lsp_types::Position {
                    line: 11,
                    character: 38,
                },
            },
        },
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/rust/src/astar.rs")?,
            range: Range {
                start: lsp_types::Position {
                    line: 1,
                    character: 17,
                },
                end: lsp_types::Position {
                    line: 1,
                    character: 21,
                },
            },
        },
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/rust/src/astar.rs")?,
            range: Range {
                start: lsp_types::Position {
                    line: 6,
                    character: 14,
                },
                end: lsp_types::Position {
                    line: 6,
                    character: 18,
                },
            },
        },
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/rust/src/astar.rs")?,
            range: Range {
                start: lsp_types::Position {
                    line: 7,
                    character: 16,
                },
                end: lsp_types::Position {
                    line: 7,
                    character: 20,
                },
            },
        },
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/rust/src/astar.rs")?,
            range: Range {
                start: lsp_types::Position {
                    line: 59,
                    character: 32,
                },
                end: lsp_types::Position {
                    line: 59,
                    character: 36,
                },
            },
        },
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/rust/src/astar.rs")?,
            range: Range {
                start: lsp_types::Position {
                    line: 76,
                    character: 35,
                },
                end: lsp_types::Position {
                    line: 76,
                    character: 39,
                },
            },
        },
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/rust/src/astar.rs")?,
            range: Range {
                start: lsp_types::Position {
                    line: 93,
                    character: 23,
                },
                end: lsp_types::Position {
                    line: 93,
                    character: 27,
                },
            },
        },
    ];
    expected.sort_by(|a, b| {
        a.uri.to_string().cmp(&b.uri.to_string()).then_with(|| {
            a.range
                .start
                .line
                .cmp(&b.range.start.line)
                .then_with(|| a.range.start.character.cmp(&b.range.start.character))
        })
    });
    assert_eq!(references, expected);
    Ok(())
}

#[tokio::test]
async fn test_definition() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&rust_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;

    sleep(Duration::from_secs(5)).await;

    let def_response = manager
        .find_definition(
            "src/node.rs",
            lsp_types::Position {
                line: 3,
                character: 11,
            },
        )
        .await?;

    let definitions = match def_response {
        GotoDefinitionResponse::Scalar(location) => vec![location],
        GotoDefinitionResponse::Array(locations) => locations,
        GotoDefinitionResponse::Link(_links) => Vec::new(),
    };
    let expected = vec![Location {
        uri: Url::parse("file:///mnt/lsproxy_root/sample_project/rust/src/node.rs")?,
        range: Range {
            start: lsp_types::Position {
                line: 3,
                character: 11,
            },
            end: lsp_types::Position {
                line: 3,
                character: 15,
            },
        },
    }];
    assert_eq!(definitions, expected);

    Ok(())
}
