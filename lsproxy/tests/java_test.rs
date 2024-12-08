use lsproxy::api_types::{
    set_global_mount_dir, FilePosition, FileRange, Position, Symbol, SymbolResponse,
};
use lsproxy::{initialize_app_state, run_server};
use reqwest;
use std::net::TcpStream;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn wait_for_server(url: &str) {
    let client = reqwest::blocking::Client::new();
    for _ in 0..30 {
        // Try for 30 seconds
        if client.get(url).send().is_ok() {
            return;
        }
        thread::sleep(Duration::from_secs(1));
    }
    panic!("Server did not respond within 30 seconds");
}

#[test]
#[ignore = "Java hangs in tests"]
fn test_server_integration_java() -> Result<(), Box<dyn std::error::Error>> {
    // Use the sample project directory directly as the mount directory
    let mount_dir = "/mnt/lsproxy_root/sample_project/java";

    let (tx, rx) = mpsc::channel();

    // Spawn the server in a separate thread
    let _server_thread = thread::spawn(move || {
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

    // Check if the server is running
    match TcpStream::connect("0.0.0.0:4444") {
        Ok(_) => println!("Server is running"),
        Err(e) => return Err(format!("Failed to connect to server: {}", e).into()),
    }
    let base_url = "http://localhost:4444";
    wait_for_server(&format!("{}/v1/workspace/list-files", base_url));

    let client = reqwest::blocking::Client::new();
    // Test workspace/list-files endpoint
    let response = client
        .get(&format!("{}/v1/workspace/list-files", base_url))
        .send()
        .expect("Failed to send request");
    assert_eq!(response.status(), 200);

    let mut workspace_files: Vec<String> = response.json().expect("Failed to parse JSON");

    // Check if the expected files are present
    let mut expected_files = vec!["AStar.java", "Main.java", "Node.java"];
    assert_eq!(
        workspace_files.len(),
        expected_files.len(),
        "Unexpected number of files"
    );

    workspace_files.sort();
    expected_files.sort();
    assert_eq!(workspace_files, expected_files, "File lists do not match");

    // Test file_symbols endpoint
    let response = client
        .get(&format!("{}/v1/symbol/definitions-in-file", base_url))
        .query(&[("file_path", "AStar.java")])
        .send()
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let returned_symbols: SymbolResponse =
        serde_json::from_value(response.json().expect("Failed to parse JSON"))?;
    let expected = vec![
        Symbol {
            name: String::from("AStar"),
            kind: String::from("class"),
            identifier_position: FilePosition {
                path: String::from("AStar.java"),
                position: Position {
                    line: 10,
                    character: 13,
                },
            },
            range: FileRange {
                path: String::from("AStar.java"),
                start: Position {
                    line: 10,
                    character: 0,
                },
                end: Position {
                    line: 96,
                    character: 21,
                },
            },
        },
        Symbol {
            name: String::from("findPathTo"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("AStar.java"),
                position: Position {
                    line: 39,
                    character: 22,
                },
            },
            range: FileRange {
                path: String::from("AStar.java"),
                start: Position {
                    line: 39,
                    character: 0,
                },
                end: Position {
                    line: 59,
                    character: 5,
                },
            },
        },
        Symbol {
            name: String::from("addNeigborsToOpenList"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("AStar.java"),
                position: Position {
                    line: 61,
                    character: 17,
                },
            },
            range: FileRange {
                path: String::from("AStar.java"),
                start: Position {
                    line: 61,
                    character: 0,
                },
                end: Position {
                    line: 89,
                    character: 41,
                },
            },
        },
        Symbol {
            name: String::from("distance"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("AStar.java"),
                position: Position {
                    line: 93,
                    character: 55,
                },
            },
            range: FileRange {
                path: String::from("AStar.java"),
                start: Position {
                    line: 93,
                    character: 0,
                },
                end: Position {
                    line: 95,
                    character: 41,
                },
            },
        },
        Symbol {
            name: String::from("main"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("AStar.java"),
                position: Position {
                    line: 98,
                    character: 59,
                },
            },
            range: FileRange {
                path: String::from("AStar.java"),
                start: Position {
                    line: 98,
                    character: 0,
                },
                end: Position {
                    line: 136,
                    character: 5,
                },
            },
        },
        Symbol {
            name: String::from("findNeighborInList"),
            kind: String::from("method"),
            identifier_position: FilePosition {
                path: String::from("AStar.java"),
                position: Position {
                    line: 138,
                    character: 20,
                },
            },
            range: FileRange {
                path: String::from("AStar.java"),
                start: Position {
                    line: 138,
                    character: 0,
                },
                end: Position {
                    line: 140,
                    character: 5,
                },
            },
        },
    ];

    assert_eq!(returned_symbols, expected);
    Ok(())
}
