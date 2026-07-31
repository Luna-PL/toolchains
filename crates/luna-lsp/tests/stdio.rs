use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Cursor, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

fn frame(message: &Value) -> Vec<u8> {
    let body = serde_json::to_vec(message).expect("message must serialize");
    let mut result = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    result.extend(body);
    result
}

fn parse_frames(bytes: Vec<u8>) -> Vec<Value> {
    let mut reader = BufReader::new(Cursor::new(bytes));
    let mut messages = Vec::new();
    loop {
        let mut length = None;
        loop {
            let mut line = String::new();
            if reader
                .read_line(&mut line)
                .expect("header must be readable")
                == 0
            {
                return messages;
            }
            if line == "\r\n" {
                break;
            }
            if let Some(value) = line.strip_prefix("Content-Length:") {
                length = Some(
                    value
                        .trim()
                        .parse::<usize>()
                        .expect("length must be numeric"),
                );
            }
        }
        let mut body = vec![0; length.expect("frame must have length")];
        reader.read_exact(&mut body).expect("body must be complete");
        messages.push(serde_json::from_slice(&body).expect("body must be JSON"));
    }
}

fn file_uri(path: &Path) -> String {
    format!("file://{}", path.to_string_lossy())
}

fn run_minimal_session() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_luna-lsp"))
        .env_remove("LUNA_BIN")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("language server must start");
    let mut input = child.stdin.take().expect("stdin must be piped");
    for message in [
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
        json!({"jsonrpc": "2.0", "id": 2, "method": "shutdown", "params": null}),
        json!({"jsonrpc": "2.0", "method": "exit", "params": null}),
    ] {
        input
            .write_all(&frame(&message))
            .expect("request must be writable");
    }
    drop(input);
    let output = child.wait_with_output().expect("language server must exit");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let messages = parse_frames(output.stdout);
    assert!(
        messages
            .iter()
            .any(|message| message.get("id") == Some(&json!(1)))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.get("id") == Some(&json!(2)))
    );
}

#[test]
fn stdio_server_restarts_without_leaking_processes() {
    for _ in 0..3 {
        run_minimal_session();
    }
}

#[test]
fn stdio_lifecycle_symbols_folding_and_optional_diagnostics() {
    let luna_bin = std::env::var_os("LUNA_BIN");
    let mut child = Command::new(env!("CARGO_BIN_EXE_luna-lsp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("language server must start");
    let mut input = child.stdin.take().expect("stdin must be piped");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/parse_missing_binding_name.luna");
    let source = std::fs::read_to_string(&fixture).expect("fixture must be readable");
    let uri = file_uri(&fixture);
    let initialization_options = luna_bin
        .as_ref()
        .map(|path| json!({"lunaPath": path.to_string_lossy()}))
        .unwrap_or(Value::Null);

    for message in [
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"initializationOptions": initialization_options}
        }),
        json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}),
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {"textDocument": {"uri": uri, "languageId": "luna", "version": 1, "text": source}}
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/documentSymbol",
            "params": {"textDocument": {"uri": uri}}
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "textDocument/foldingRange",
            "params": {"textDocument": {"uri": uri}}
        }),
    ] {
        input
            .write_all(&frame(&message))
            .expect("request must be writable");
    }
    input.flush().expect("requests must flush");
    thread::sleep(Duration::from_millis(350));
    input
        .write_all(&frame(&json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "shutdown",
            "params": null
        })))
        .expect("shutdown must be writable");
    input
        .write_all(&frame(&json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        })))
        .expect("exit must be writable");
    drop(input);

    let output = child.wait_with_output().expect("language server must exit");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let messages = parse_frames(output.stdout);
    assert!(messages.iter().any(|message| {
        message.get("id") == Some(&json!(1))
            && message.pointer("/result/capabilities/textDocumentSync/change") == Some(&json!(2))
    }));
    assert!(
        messages.iter().any(|message| {
            message.get("id") == Some(&json!(2)) && message["result"].is_array()
        })
    );
    assert!(
        messages.iter().any(|message| {
            message.get("id") == Some(&json!(3)) && message["result"].is_array()
        })
    );
    assert!(
        messages
            .iter()
            .any(|message| { message.get("id") == Some(&json!(4)) && message["result"].is_null() })
    );
    if luna_bin.is_some() {
        assert!(messages.iter().any(|message| {
            message.get("method") == Some(&json!("textDocument/publishDiagnostics"))
                && message
                    .pointer("/params/diagnostics/0/code")
                    .is_some_and(|code| code == "PAR0001")
        }));
    }
}
