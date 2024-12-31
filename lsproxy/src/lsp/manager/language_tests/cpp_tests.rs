use super::*;

#[tokio::test]
async fn test_file_symbols() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&cpp_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;

    let file_path = "cpp_classes/astar.cpp";
    let file_symbols = manager.definitions_in_file_ast_grep(file_path).await?;
    let symbol_response: SymbolResponse =
        file_symbols.into_iter().map(|s| Symbol::from(s)).collect();

    let expected = vec![
        Symbol {
            name: String::from("aStar"),
            kind: String::from("class"),
            identifier_position: FilePosition {
                path: String::from("cpp_classes/astar.cpp"),
                position: Position {
                    line: 8,
                    character: 6,
                },
            },
            range: FileRange {
                path: String::from("cpp_classes/astar.cpp"),
                start: Position {
                    line: 8,
                    character: 0,
                },
                end: Position {
                    line: 101,
                    character: 1,
                },
            },
        },
        Symbol {
            name: String::from("aStar"),
            kind: String::from("function-definition"),
            identifier_position: FilePosition {
                path: String::from("cpp_classes/astar.cpp"),
                position: Position {
                    line: 10,
                    character: 4,
                },
            },
            range: FileRange {
                path: String::from("cpp_classes/astar.cpp"),
                start: Position {
                    line: 10,
                    character: 0,
                },
                end: Position {
                    line: 15,
                    character: 5,
                },
            },
        },
        Symbol {
            name: String::from("calcDist"),
            kind: String::from("function-definition"),
            identifier_position: FilePosition {
                path: String::from("cpp_classes/astar.cpp"),
                position: Position {
                    line: 17,
                    character: 8,
                },
            },
            range: FileRange {
                path: String::from("cpp_classes/astar.cpp"),
                start: Position {
                    line: 17,
                    character: 0,
                },
                end: Position {
                    line: 21,
                    character: 5,
                },
            },
        },
        Symbol {
            name: String::from("isValid"),
            kind: String::from("function-definition"),
            identifier_position: FilePosition {
                path: String::from("cpp_classes/astar.cpp"),
                position: Position {
                    line: 23,
                    character: 9,
                },
            },
            range: FileRange {
                path: String::from("cpp_classes/astar.cpp"),
                start: Position {
                    line: 23,
                    character: 0,
                },
                end: Position {
                    line: 25,
                    character: 5,
                },
            },
        },
        Symbol {
            name: String::from("existPoint"),
            kind: String::from("function-definition"),
            identifier_position: FilePosition {
                path: String::from("cpp_classes/astar.cpp"),
                position: Position {
                    line: 27,
                    character: 9,
                },
            },
            range: FileRange {
                path: String::from("cpp_classes/astar.cpp"),
                start: Position {
                    line: 27,
                    character: 0,
                },
                end: Position {
                    line: 40,
                    character: 5,
                },
            },
        },
        Symbol {
            name: String::from("fillOpen"),
            kind: String::from("function-definition"),
            identifier_position: FilePosition {
                path: String::from("cpp_classes/astar.cpp"),
                position: Position {
                    line: 42,
                    character: 9,
                },
            },
            range: FileRange {
                path: String::from("cpp_classes/astar.cpp"),
                start: Position {
                    line: 42,
                    character: 0,
                },
                end: Position {
                    line: 65,
                    character: 5,
                },
            },
        },
        Symbol {
            name: String::from("search"),
            kind: String::from("function-definition"),
            identifier_position: FilePosition {
                path: String::from("cpp_classes/astar.cpp"),
                position: Position {
                    line: 67,
                    character: 9,
                },
            },
            range: FileRange {
                path: String::from("cpp_classes/astar.cpp"),
                start: Position {
                    line: 67,
                    character: 0,
                },
                end: Position {
                    line: 79,
                    character: 5,
                },
            },
        },
        Symbol {
            name: String::from("path"),
            kind: String::from("function-definition"),
            identifier_position: FilePosition {
                path: String::from("cpp_classes/astar.cpp"),
                position: Position {
                    line: 81,
                    character: 8,
                },
            },
            range: FileRange {
                path: String::from("cpp_classes/astar.cpp"),
                start: Position {
                    line: 81,
                    character: 0,
                },
                end: Position {
                    line: 95,
                    character: 5,
                },
            },
        },
    ];

    assert_eq!(symbol_response, expected);
    Ok(())
}
