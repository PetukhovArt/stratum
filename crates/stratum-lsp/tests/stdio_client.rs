#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/tiny-ts-violations")
        .canonicalize()
        .unwrap()
}

fn client_uri(path: &Path) -> String {
    let s = path.to_string_lossy();
    let s = s.strip_prefix(r"\\?\").unwrap_or(&s).replace('\\', "/");
    match s.split_once(':') {
        Some((drive, rest)) if drive.len() == 1 => {
            format!("file:///{}%3A{rest}", drive.to_lowercase())
        }
        _ => format!("file://{s}"),
    }
}

fn send(stdin: &mut ChildStdin, msg: &Value) {
    let body = msg.to_string();
    write!(stdin, "Content-Length: {}\r\n\r\n{body}", body.len()).unwrap();
    stdin.flush().unwrap();
}

fn spawn_reader(child: &mut Child) -> Receiver<Value> {
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        loop {
            let mut len = None;
            loop {
                let mut line = String::new();
                if stdout.read_line(&mut line).unwrap_or(0) == 0 {
                    return;
                }
                let line = line.trim_end();
                if line.is_empty() {
                    break;
                }
                let (name, value) = line.split_once(": ").expect("LSP header on stdout");
                if name.eq_ignore_ascii_case("Content-Length") {
                    len = Some(value.parse::<usize>().unwrap());
                }
            }
            let mut body = vec![0; len.expect("Content-Length header")];
            stdout.read_exact(&mut body).unwrap();
            let msg: Value = serde_json::from_slice(&body).expect("JSON-RPC body on stdout");
            assert_eq!(msg["jsonrpc"], "2.0");
            if tx.send(msg).is_err() {
                return;
            }
        }
    });
    rx
}

#[test]
fn publishes_slug_coded_diagnostics_over_stdio() {
    let root = fixture_root();
    let mut child = Command::new(env!("CARGO_BIN_EXE_stratum-lsp"))
        .current_dir(std::env::temp_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let rx = spawn_reader(&mut child);
    let mut stdin = child.stdin.take().unwrap();

    send(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": { "processId": null, "rootUri": client_uri(&root), "capabilities": {} }
        }),
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let msg = rx
            .recv_timeout(deadline - Instant::now())
            .expect("initialize response");
        if msg["id"] == 1 {
            assert!(msg["result"]["capabilities"].is_object(), "{msg}");
            break;
        }
    }
    send(
        &mut stdin,
        &json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }),
    );

    let expected = [
        ("src/shared/api/http.ts", "stratum/no-cross-layer-import"),
        ("src/features/cart/circle.ts", "stratum/no-circular-deps"),
        ("src/features/cart/boom.ts", "stratum/no-circular-deps"),
    ];
    for (rel, _) in expected {
        let path = root.join(rel);
        send(
            &mut stdin,
            &json!({
                "jsonrpc": "2.0", "method": "textDocument/didOpen",
                "params": { "textDocument": {
                    "uri": client_uri(&path), "languageId": "typescript", "version": 1,
                    "text": std::fs::read_to_string(&path).unwrap()
                }}
            }),
        );
    }

    let mut latest: HashMap<String, Vec<Value>> = HashMap::new();
    let has = |latest: &HashMap<String, Vec<Value>>, rel: &str, code: &str| {
        latest.get(&client_uri(&root.join(rel))).is_some_and(|ds| {
            ds.iter().any(|d| {
                d["code"] == code
                    && d["source"] == "stratum"
                    && d["relatedInformation"]
                        .as_array()
                        .is_some_and(|r| !r.is_empty())
            })
        })
    };
    let deadline = Instant::now() + Duration::from_secs(60);
    while !expected.iter().all(|(rel, code)| has(&latest, rel, code)) {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let msg = rx
            .recv_timeout(remaining)
            .unwrap_or_else(|_| panic!("missing diagnostics, got {latest:?}"));
        if msg["method"] == "textDocument/publishDiagnostics" {
            let uri = msg["params"]["uri"].as_str().unwrap().to_string();
            let diags = msg["params"]["diagnostics"].as_array().unwrap().clone();
            latest.insert(uri, diags);
        }
    }

    send(
        &mut stdin,
        &json!({ "jsonrpc": "2.0", "id": 2, "method": "shutdown" }),
    );
    send(&mut stdin, &json!({ "jsonrpc": "2.0", "method": "exit" }));
    let deadline = Instant::now() + Duration::from_secs(5);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() > deadline {
            let _ = child.kill();
            panic!("server still running after exit while stdin is open");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}
