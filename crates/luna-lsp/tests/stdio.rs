use luna_compiler::{CandidateSource, probe};
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

#[test]
fn optional_real_compiler_definition_follows_resolved_symbol_id() {
    let Some(luna_bin) = std::env::var_os("LUNA_BIN") else {
        return;
    };
    let mut child = Command::new(env!("CARGO_BIN_EXE_luna-lsp"))
        .env("LUNA_BIN", &luna_bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("language server must start");
    let mut input = child.stdin.take().expect("stdin must be piped");
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/definition.luna");
    let source = std::fs::read_to_string(&fixture).expect("fixture must be readable");
    let dirty_source_v2 = format!("// first unsaved edit\n{source}");
    let dirty_source_v3 = format!("// first unsaved edit\n// 月 second edit\n{source}");
    let uri = file_uri(&fixture);

    for message in [
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"initializationOptions": {"lunaPath": luna_bin.to_string_lossy()}}
        }),
        json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}),
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": uri, "languageId": "luna", "version": 1, "text": source.clone()
            }}
        }),
    ] {
        input
            .write_all(&frame(&message))
            .expect("request must be writable");
    }
    input.flush().expect("requests must flush");
    thread::sleep(Duration::from_millis(350));
    for message in [
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/definition",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": 16, "character": 12}
            }
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "textDocument/definition",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": 16, "character": 21}
            }
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "textDocument/definition",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": 6, "character": 23}
            }
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "textDocument/definition",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": 12, "character": 7}
            }
        }),
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {"uri": uri, "version": 2},
                "contentChanges": [{"text": dirty_source_v2}]
            }
        }),
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {"uri": uri, "version": 3},
                "contentChanges": [{"text": dirty_source_v3}]
            }
        }),
    ] {
        input
            .write_all(&frame(&message))
            .expect("request must be writable");
    }
    input.flush().expect("dirty requests must flush");
    thread::sleep(Duration::from_millis(350));
    for message in [
        json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "textDocument/definition",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": 18, "character": 21}
            }
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "textDocument/definition",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": 8, "character": 23}
            }
        }),
        json!({"jsonrpc": "2.0", "id": 8, "method": "shutdown", "params": null}),
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
    assert!(messages.iter().any(|message| {
        message.get("id") == Some(&json!(1))
            && message.pointer("/result/capabilities/definitionProvider") == Some(&json!(true))
    }));
    assert!(messages.iter().any(|message| {
        message.get("id") == Some(&json!(2))
            && message.pointer("/result/range/start/line") == Some(&json!(0))
            && message.pointer("/result/range/start/character") == Some(&json!(3))
    }));
    assert!(messages.iter().any(|message| {
        message.get("id") == Some(&json!(3))
            && message.pointer("/result/range/start/line") == Some(&json!(13))
            && message.pointer("/result/range/start/character") == Some(&json!(7))
    }));
    assert!(messages.iter().any(|message| {
        message.get("id") == Some(&json!(4))
            && message.pointer("/result/range/start/line") == Some(&json!(3))
            && message.pointer("/result/range/start/character") == Some(&json!(15))
    }));
    assert!(messages.iter().any(|message| {
        message.get("id") == Some(&json!(5))
            && message.pointer("/result/range/start/line") == Some(&json!(9))
            && message.pointer("/result/range/start/character") == Some(&json!(6))
    }));
    assert!(messages.iter().any(|message| {
        message.get("id") == Some(&json!(6))
            && message.pointer("/result/range/start/line") == Some(&json!(15))
            && message.pointer("/result/range/start/character") == Some(&json!(7))
    }));
    assert!(messages.iter().any(|message| {
        message.get("id") == Some(&json!(7))
            && message.pointer("/result/range/start/line") == Some(&json!(5))
            && message.pointer("/result/range/start/character") == Some(&json!(15))
    }));
}

#[test]
fn optional_real_compiler_combines_dirty_package_documents() {
    let Some(luna_bin) = std::env::var_os("LUNA_BIN") else {
        return;
    };
    let compiler = probe(Path::new(&luna_bin), CandidateSource::Explicit)
        .expect("LUNA_BIN must implement the diagnostic protocol");
    if ![
        "multi-document-overlay",
        "package-references",
        "call-references",
    ]
    .iter()
    .all(|expected| {
        compiler
            .identity
            .analysis_capabilities
            .iter()
            .any(|available| available == expected)
    }) {
        return;
    }
    let mut child = Command::new(env!("CARGO_BIN_EXE_luna-lsp"))
        .env("LUNA_BIN", &luna_bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("language server must start");
    let mut input = child.stdin.take().expect("stdin must be piped");
    let package = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/multi_overlay");
    let api = package.join("src/01_api.luna");
    let main = package.join("src/02_main.luna");
    let api_uri = file_uri(&api);
    let main_uri = file_uri(&main);
    let api_v1 = "package org.luna.toolchain.multi_overlay;\nmodule api;\n// 月 first\nexport fn moon_answer() -> i32 { return 42; }\n";
    let main_v1 = "package org.luna.toolchain.multi_overlay;\nmodule application;\n\nfn main() -> i32 {\n    return api::moon_answer();\n}\n";

    for message in [
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"initializationOptions": {"lunaPath": luna_bin.to_string_lossy()}}
        }),
        json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}),
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": api_uri, "languageId": "luna", "version": 1, "text": api_v1
            }}
        }),
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": main_uri, "languageId": "luna", "version": 1, "text": main_v1
            }}
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
            "id": 2,
            "method": "textDocument/definition",
            "params": {
                "textDocument": {"uri": main_uri},
                "position": {"line": 4, "character": 18}
            }
        })))
        .expect("definition request must be writable");

    let api_v2 = api_v1.replace("moon_answer", "star_answer");
    let main_v2 = main_v1.replace("moon_answer", "star_answer");
    let api_v3 = api_v1
        .replace("月 first", "星 final")
        .replace("moon_answer", "solar_answer");
    let main_v3 = main_v1.replace("moon_answer", "solar_answer");
    for message in [
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didChange",
            "params": {"textDocument": {"uri": api_uri, "version": 2},
                       "contentChanges": [{"text": api_v2}]}
        }),
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didChange",
            "params": {"textDocument": {"uri": main_uri, "version": 2},
                       "contentChanges": [{"text": main_v2}]}
        }),
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didChange",
            "params": {"textDocument": {"uri": api_uri, "version": 3},
                       "contentChanges": [{"text": api_v3}]}
        }),
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didChange",
            "params": {"textDocument": {"uri": main_uri, "version": 3},
                       "contentChanges": [{"text": main_v3}]}
        }),
    ] {
        input
            .write_all(&frame(&message))
            .expect("dirty request must be writable");
    }
    input.flush().expect("dirty requests must flush");
    thread::sleep(Duration::from_millis(350));
    for message in [
        json!({
            "jsonrpc": "2.0", "id": 3, "method": "textDocument/definition",
            "params": {"textDocument": {"uri": main_uri},
                       "position": {"line": 4, "character": 18}}
        }),
        json!({
            "jsonrpc": "2.0", "id": 4, "method": "textDocument/documentSymbol",
            "params": {"textDocument": {"uri": api_uri}}
        }),
        json!({
            "jsonrpc": "2.0", "id": 5, "method": "textDocument/references",
            "params": {"textDocument": {"uri": api_uri},
                       "position": {"line": 3, "character": 12},
                       "context": {"includeDeclaration": true}}
        }),
        json!({
            "jsonrpc": "2.0", "id": 6, "method": "textDocument/references",
            "params": {"textDocument": {"uri": main_uri},
                       "position": {"line": 4, "character": 18},
                       "context": {"includeDeclaration": false}}
        }),
        json!({
            "jsonrpc": "2.0", "id": 7, "method": "textDocument/prepareRename",
            "params": {"textDocument": {"uri": main_uri},
                       "position": {"line": 4, "character": 18}}
        }),
        json!({
            "jsonrpc": "2.0", "id": 8, "method": "textDocument/rename",
            "params": {"textDocument": {"uri": main_uri},
                       "position": {"line": 4, "character": 18},
                       "newName": "lunar_answer"}
        }),
        json!({"jsonrpc": "2.0", "id": 9, "method": "shutdown", "params": null}),
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
    assert!(messages.iter().any(|message| {
        message.get("id") == Some(&json!(1))
            && message.pointer("/result/capabilities/referencesProvider") == Some(&json!(true))
            && message.pointer("/result/capabilities/renameProvider/prepareProvider")
                == Some(&json!(true))
    }));
    for id in [2, 3] {
        assert!(messages.iter().any(|message| {
            message.get("id") == Some(&json!(id))
                && message.pointer("/result/uri") == Some(&json!(api_uri))
                && message.pointer("/result/range/start/line") == Some(&json!(3))
                && message.pointer("/result/range/start/character") == Some(&json!(10))
        }));
    }
    assert!(messages.iter().any(|message| {
        message.get("id") == Some(&json!(4))
            && message
                .get("result")
                .and_then(Value::as_array)
                .is_some_and(|symbols| {
                    symbols
                        .iter()
                        .any(|symbol| symbol["name"] == "solar_answer")
                })
    }));
    assert!(messages.iter().any(|message| {
        message.get("id") == Some(&json!(5))
            && message
                .get("result")
                .and_then(Value::as_array)
                .is_some_and(|locations| {
                    locations.len() == 2
                        && locations[0]["uri"] == api_uri
                        && locations[1]["uri"] == main_uri
                })
    }));
    assert!(messages.iter().any(|message| {
        message.get("id") == Some(&json!(6))
            && message
                .get("result")
                .and_then(Value::as_array)
                .is_some_and(|locations| locations.len() == 1 && locations[0]["uri"] == main_uri)
    }));
    assert!(messages.iter().any(|message| {
        message.get("id") == Some(&json!(7))
            && message.pointer("/result/placeholder") == Some(&json!("solar_answer"))
    }));
    assert!(messages.iter().any(|message| {
        message.get("id") == Some(&json!(8))
            && message
                .pointer("/result/changes")
                .and_then(Value::as_object)
                .is_some_and(|changes| {
                    changes
                        .get(&api_uri)
                        .and_then(Value::as_array)
                        .is_some_and(|edits| {
                            edits.len() == 1 && edits[0]["newText"] == "lunar_answer"
                        })
                        && changes
                            .get(&main_uri)
                            .and_then(Value::as_array)
                            .is_some_and(|edits| {
                                edits.len() == 1 && edits[0]["newText"] == "lunar_answer"
                            })
                })
    }));
}
