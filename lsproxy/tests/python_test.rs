use lsproxy::api_types::{
    set_global_mount_dir, FilePosition, FileRange, HealthResponse, Position, Symbol, SymbolResponse,
};
use lsproxy::{initialize_app_state, run_server};
use reqwest;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn wait_for_server(base_url: &str) {
    let client = reqwest::blocking::Client::new();
    let health_url = format!("{}/v1/system/health", base_url);

    for _ in 0..30 {
        // Try for 30 seconds
        if let Ok(response) = client.get(&health_url).send() {
            if let Ok(health) = response.json::<HealthResponse>() {
                if health.status == "ok" {
                    return;
                }
            }
        }
        thread::sleep(Duration::from_secs(1));
    }
    panic!("Server did not respond with healthy status within 30 seconds");
}

#[test]
fn test_server_integration_python() -> Result<(), Box<dyn std::error::Error>> {
    // Use the sample project directory directly as the mount directory
    let mount_dir = "/mnt/lsproxy_root/sample_project/python";

    let (tx, rx) = mpsc::channel();

    // Spawn the server in a separate thread
    let _server_thread = thread::spawn(move || {
        std::env::set_var("USE_AUTH", "false");
        set_global_mount_dir(&mount_dir);

        let system = actix_web::rt::System::new();
        if let Err(e) = system.block_on(async {
            match initialize_app_state().await {
                Ok(app_state) => run_server(app_state).await,
                Err(e) => {
                    tx.send(format!("Failed to initialize app state: {}", e))
                        .unwrap();
                    Ok(())
                }
            }
        }) {
            tx.send(format!("System error: {}", e)).unwrap();
        }
    });

    // Give the server some time to start
    thread::sleep(Duration::from_secs(5));

    // Check for any errors from the server thread
    if let Ok(error_msg) = rx.try_recv() {
        return Err(error_msg.into());
    }

    let base_url = "http://localhost:4444";
    wait_for_server(base_url);

    let client = reqwest::blocking::Client::new();
    // Test workspace/list-files endpoint
    let response = client
        .get(&format!("{}/v1/workspace/list-files", base_url))
        .send()
        .expect("Failed to send request");
    assert_eq!(response.status(), 200);

    let mut workspace_files: Vec<String> = response.json().expect("Failed to parse JSON");

    // Check if the expected files are present
    let mut expected_files = vec!["graph.py", "main.py", "search.py", "__init__.py"];
    assert_eq!(
        workspace_files.len(),
        expected_files.len(),
        "Unexpected number of files"
    );

    workspace_files.sort();
    expected_files.sort();
    assert_eq!(workspace_files, expected_files, "File lists do not match");

    // Test read_source_code endpoint - full file
    let response = client
        .post(&format!("{}/v1/workspace/read-source-code", base_url))
        .json(&lsproxy::api_types::ReadSourceCodeRequest {
            path: "main.py".to_string(),
            range: None,
        })
        .send()
        .expect("Failed to send request");
    assert_eq!(response.status(), 200);
    let read_response: lsproxy::api_types::ReadSourceCodeResponse = response.json().expect("Failed to parse JSON");
    assert!(!read_response.source_code.is_empty(), "Source code should not be empty");

    // Test read_source_code endpoint - with range
    let response = client
        .post(&format!("{}/v1/workspace/read-source-code", base_url))
        .json(&lsproxy::api_types::ReadSourceCodeRequest {
            path: "main.py".to_string(),
            range: Some(FileRange {
                path: "main.py".to_string(),
                start: Position { line: 5, character: 0 },
                end: Position { line: 5, character: 20 },
            }),
        })
        .send()
        .expect("Failed to send request");
    assert_eq!(response.status(), 200);
    let read_response: lsproxy::api_types::ReadSourceCodeResponse = response.json().expect("Failed to parse JSON");
    assert_eq!(read_response.source_code, "graph = Graph(edges)", "Range read returned unexpected content");

    // Test read_source_code endpoint - invalid path
    let response = client
        .post(&format!("{}/v1/workspace/read-source-code", base_url))
        .json(&lsproxy::api_types::ReadSourceCodeRequest {
            path: "nonexistent.py".to_string(),
            range: None,
        })
        .send()
        .expect("Failed to send request");
    assert_eq!(response.status(), 400, "Should return 400 for nonexistent file");

    // Test file_symbols endpoint
    let response = client
        .get(&format!("{}/v1/symbol/definitions-in-file", base_url))
        .query(&[("file_path", "main.py")])
        .send()
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let returned_symbols: SymbolResponse =
        serde_json::from_value(response.json().expect("Failed to parse JSON"))?;
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
    assert_eq!(returned_symbols, expected);
    Ok(())
}
