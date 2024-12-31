use super::*;

#[tokio::test]
async fn test_references() -> Result<(), Box<dyn std::error::Error>> {
    let context = TestContext::setup(&c_sample_path(), true).await?;
    let manager = context
        .manager
        .as_ref()
        .ok_or("Manager is not initialized")?;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let references = manager
        .find_references(
            "map.c",
            lsp_types::Position {
                line: 30,
                character: 5,
            },
        )
        .await?;

    let expected = vec![
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/c/map.c").unwrap(),
            range: lsp_types::Range {
                start: lsp_types::Position {
                    line: 30,
                    character: 5,
                },
                end: lsp_types::Position {
                    line: 30,
                    character: 14,
                },
            },
        },
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/c/main.c").unwrap(),
            range: Range {
                start: lsp_types::Position {
                    line: 15,
                    character: 8,
                },
                end: lsp_types::Position {
                    line: 15,
                    character: 17,
                },
            },
        },
        Location {
            uri: Url::parse("file:///mnt/lsproxy_root/sample_project/c/map.h").unwrap(),
            range: Range {
                start: lsp_types::Position {
                    line: 11,
                    character: 5,
                },
                end: lsp_types::Position {
                    line: 11,
                    character: 14,
                },
            },
        },
    ];

    // Sort locations before comparing
    let mut actual_locations = references;
    let mut expected_locations = expected;

    actual_locations.sort_by(|a, b| a.uri.path().cmp(&b.uri.path()));
    expected_locations.sort_by(|a, b| a.uri.path().cmp(&b.uri.path()));

    assert_eq!(actual_locations, expected_locations);
    Ok(())
}
