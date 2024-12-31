use super::*;

#[tokio::test]
async fn test_file_symbols() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&typescript_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;
    let file_path = "PathfinderDisplay.tsx";
    let file_symbols = manager.definitions_in_file_ast_grep(file_path).await?;
    let mut symbol_response: SymbolResponse =
        file_symbols.into_iter().map(|s| Symbol::from(s)).collect();

    let mut expected = vec![
        Symbol {
            name: String::from("PathfinderDisplay"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("PathfinderDisplay.tsx"),
                position: Position {
                    line: 12,
                    character: 13,
                },
            },
            range: FileRange {
                path: String::from("PathfinderDisplay.tsx"),
                start: Position {
                    line: 12,
                    character: 0,
                },
                end: Position {
                    line: 125,
                    character: 1,
                },
            },
        },
        Symbol {
            name: String::from("PathfinderDisplayProps"),
            kind: String::from("interface"),
            identifier_position: FilePosition {
                path: String::from("PathfinderDisplay.tsx"),
                position: Position {
                    line: 5,
                    character: 10,
                },
            },
            range: FileRange {
                path: String::from("PathfinderDisplay.tsx"),
                start: Position {
                    line: 5,
                    character: 0,
                },
                end: Position {
                    line: 10,
                    character: 1,
                },
            },
        },
        Symbol {
            name: String::from("findPath"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("PathfinderDisplay.tsx"),
                position: Position {
                    line: 32,
                    character: 10,
                },
            },
            range: FileRange {
                path: String::from("PathfinderDisplay.tsx"),
                start: Position {
                    line: 32,
                    character: 0,
                },
                end: Position {
                    line: 38,
                    character: 5,
                },
            },
        },
        Symbol {
            name: String::from("getCellColor"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("PathfinderDisplay.tsx"),
                position: Position {
                    line: 52,
                    character: 10,
                },
            },
            range: FileRange {
                path: String::from("PathfinderDisplay.tsx"),
                start: Position {
                    line: 52,
                    character: 0,
                },
                end: Position {
                    line: 61,
                    character: 5,
                },
            },
        },
        Symbol {
            name: String::from("toggleCell"),
            kind: String::from("function"),
            identifier_position: FilePosition {
                path: String::from("PathfinderDisplay.tsx"),
                position: Position {
                    line: 63,
                    character: 10,
                },
            },
            range: FileRange {
                path: String::from("PathfinderDisplay.tsx"),
                start: Position {
                    line: 63,
                    character: 0,
                },
                end: Position {
                    line: 71,
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
