//! Drives `t lsp` the way an editor would: JSON-RPC messages over stdio, each
//! framed with a `Content-Length` header.

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{ChildStdin, ChildStdout, Command, Stdio};

struct Client {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Client {
    fn send(&mut self, msg: Value) {
        let body = msg.to_string();
        write!(self.stdin, "Content-Length: {}\r\n\r\n{body}", body.len()).unwrap();
        self.stdin.flush().unwrap();
    }

    fn recv(&mut self) -> Value {
        let mut len = None;
        loop {
            let mut line = String::new();
            self.stdout.read_line(&mut line).unwrap();
            let line = line.trim_end();
            if line.is_empty() {
                break;
            }
            if let Some(n) = line.strip_prefix("Content-Length: ") {
                len = Some(n.parse().unwrap());
            }
        }
        let mut body = vec![0; len.expect("message without Content-Length")];
        self.stdout.read_exact(&mut body).unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    /// Skips anything else until the response to request `id`.
    fn response(&mut self, id: i64) -> Value {
        loop {
            let msg = self.recv();
            if msg["id"] == id {
                return msg;
            }
        }
    }

    /// Skips anything else until the next `publishDiagnostics` notification.
    fn diagnostics(&mut self) -> Vec<Value> {
        loop {
            let msg = self.recv();
            if msg["method"] == "textDocument/publishDiagnostics" {
                return msg["params"]["diagnostics"].as_array().unwrap().clone();
            }
        }
    }
}

/// Starts `t lsp` and initializes it.
fn start() -> (std::process::Child, Client) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_t"))
        .arg("lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut c = Client {
        stdin: child.stdin.take().unwrap(),
        stdout: BufReader::new(child.stdout.take().unwrap()),
    };
    c.send(
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"capabilities": {}}}),
    );
    let init = c.recv();
    let caps = &init["result"]["capabilities"];
    assert_eq!(caps["textDocumentSync"], 1, "{init}");
    assert_eq!(
        caps["codeActionProvider"]["codeActionKinds"],
        json!(["quickfix"]),
        "{init}"
    );
    assert_eq!(caps["documentFormattingProvider"], true, "{init}");
    c.send(json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}));
    (child, c)
}

fn stop(mut child: std::process::Child, mut c: Client) {
    c.send(json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown"}));
    assert_eq!(c.response(99)["id"], 99);
    c.send(json!({"jsonrpc": "2.0", "method": "exit"}));
    assert!(child.wait().unwrap().success());
}

#[test]
fn reports_and_clears_diagnostics() {
    let (child, mut c) = start();
    let uri = "file:///tmp/test.t";

    // Open a file with a type error on line 2 (0-based line 1).
    let bad = "fn main() {\n    let x: int = true;\n}\n";
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": uri, "languageId": "t", "version": 1, "text": bad}
        }}),
    );
    let diags = c.diagnostics();
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(
        diags[0]["message"],
        "type error: expected `int`, found `bool`"
    );
    assert_eq!(
        diags[0]["range"]["start"],
        json!({"line": 1, "character": 17})
    );
    assert_eq!(
        diags[0]["range"]["end"],
        json!({"line": 1, "character": 21})
    );

    // Fix it: the diagnostics are cleared.
    let good = "fn main() {\n    let _x: int = 1;\n}\n";
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
            "textDocument": {"uri": uri, "version": 2},
            "contentChanges": [{"text": good}]
        }}),
    );
    assert_eq!(c.diagnostics(), Vec::<Value>::new());

    stop(child, c);
}

#[test]
fn warns_and_offers_quick_fixes() {
    let (child, mut c) = start();
    let uri = "file:///tmp/warn.t";

    // The `false` arm (0-based line 4) can never run.
    let src = "fn f(b: bool) -> int {\n    match b {\n        true => 1,\n        _ => 2,\n        false => 3,\n    }\n}\nfn main() {\n    print(f(true));\n}\n";
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": uri, "languageId": "t", "version": 1, "text": src}
        }}),
    );
    let diags = c.diagnostics();
    assert_eq!(diags.len(), 1, "{diags:?}");
    let d = &diags[0];
    assert_eq!(d["severity"], 2, "a warning: {d}");
    assert_eq!(d["code"], "unreachable_patterns");
    assert_eq!(d["tags"], json!([1]), "tagged unnecessary: {d}");
    let range = json!({"start": {"line": 4, "character": 8}, "end": {"line": 4, "character": 13}});
    assert_eq!(d["range"], range);

    // Ask for code actions with the cursor on the arm.
    let cursor = json!({"line": 4, "character": 10});
    c.send(
        json!({"jsonrpc": "2.0", "id": 2, "method": "textDocument/codeAction", "params": {
            "textDocument": {"uri": uri},
            "range": {"start": cursor, "end": cursor},
            "context": {"diagnostics": [d]}
        }}),
    );
    let actions = c.response(2)["result"].clone();
    assert_eq!(actions.as_array().unwrap().len(), 1, "{actions}");
    let action = &actions[0];
    assert_eq!(action["title"], "remove the unreachable arm");
    assert_eq!(action["kind"], "quickfix");
    assert_eq!(action["isPreferred"], true);
    // From the end of `2` to the end of `3`, leaving the trailing `,`.
    assert_eq!(
        action["edit"]["changes"][uri],
        json!([{
            "range": {"start": {"line": 3, "character": 14}, "end": {"line": 4, "character": 18}},
            "newText": ""
        }])
    );

    // Nothing to offer elsewhere.
    let elsewhere = json!({"line": 0, "character": 0});
    c.send(
        json!({"jsonrpc": "2.0", "id": 3, "method": "textDocument/codeAction", "params": {
            "textDocument": {"uri": uri},
            "range": {"start": elsewhere, "end": elsewhere},
            "context": {"diagnostics": []}
        }}),
    );
    assert_eq!(c.response(3)["result"], json!([]));

    stop(child, c);
}

#[test]
fn hovers_hints_and_goes_to_definitions() {
    let (child, mut c) = start();
    let uri = "file:///tmp/generic.t";
    let src = "fn id<T>(x: T) -> T { x }\nfn main() {\n    let n = id(1);\n    print(n);\n    print(id::<int>(2));\n}\n";
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": uri, "languageId": "t", "version": 1, "text": src}
        }}),
    );
    assert_eq!(c.diagnostics(), Vec::<Value>::new());
    let at = |line: u32, character: u32| json!({"textDocument": {"uri": uri}, "position": {"line": line, "character": character}});

    // `id` in `id(1)` is the copy of `id` for `int`.
    c.send(json!({"jsonrpc": "2.0", "id": 2, "method": "textDocument/hover", "params": at(2, 13)}));
    let hover = c.response(2)["result"].clone();
    assert_eq!(
        hover["contents"]["value"], "```t\nfn id::<int>: fn(int) -> int\n```",
        "{hover}"
    );
    // Inside `id`, `x` is a `T`.
    c.send(json!({"jsonrpc": "2.0", "id": 3, "method": "textDocument/hover", "params": at(0, 22)}));
    let hover = c.response(3)["result"].clone();
    assert_eq!(hover["contents"]["value"], "```t\nx: T\n```", "{hover}");

    // `n` is defined by the `let`, `id` by the `fn`.
    c.send(json!({"jsonrpc": "2.0", "id": 4, "method": "textDocument/definition", "params": at(3, 10)}));
    let def = c.response(4)["result"].clone();
    assert_eq!(def["uri"], uri);
    assert_eq!(
        def["range"],
        json!({"start": {"line": 2, "character": 8}, "end": {"line": 2, "character": 9}})
    );
    c.send(json!({"jsonrpc": "2.0", "id": 5, "method": "textDocument/definition", "params": at(2, 12)}));
    let def = c.response(5)["result"].clone();
    assert_eq!(
        def["range"],
        json!({"start": {"line": 0, "character": 3}, "end": {"line": 0, "character": 5}})
    );

    // The type of `n` and the type argument of `id`, both inferred. Not the
    // one written out in `id::<int>(2)`.
    let whole = json!({"start": {"line": 0, "character": 0}, "end": {"line": 6, "character": 0}});
    c.send(
        json!({"jsonrpc": "2.0", "id": 6, "method": "textDocument/inlayHint", "params": {
            "textDocument": {"uri": uri}, "range": whole
        }}),
    );
    let hints = c.response(6)["result"].clone();
    let hints: Vec<(Value, Value)> = hints
        .as_array()
        .unwrap()
        .iter()
        .map(|h| (h["position"].clone(), h["label"].clone()))
        .collect();
    assert_eq!(
        hints,
        vec![
            (json!({"line": 2, "character": 9}), json!(": int")),
            (json!({"line": 2, "character": 14}), json!("::<int>")),
        ]
    );

    stop(child, c);
}

#[test]
fn formats_documents() {
    let (child, mut c) = start();
    let uri = "file:///tmp/fmt.t";
    let open = |c: &mut Client, text: &str| {
        c.send(
            json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
                "textDocument": {"uri": uri, "languageId": "t", "version": 1, "text": text}
            }}),
        );
        c.diagnostics();
    };
    // The editor's options are ignored: there's one style.
    let format = |c: &mut Client, id: i64| {
        c.send(
            json!({"jsonrpc": "2.0", "id": id, "method": "textDocument/formatting", "params": {
                "textDocument": {"uri": uri},
                "options": {"tabSize": 2, "insertSpaces": false}
            }}),
        );
        c.response(id)
    };

    // One edit, replacing the whole file.
    let messy = "fn main()   {\n  print(1) // expect: 1\n}\n\n";
    open(&mut c, messy);
    let response = format(&mut c, 2);
    assert_eq!(
        response["result"],
        json!([{
            "range": {"start": {"line": 0, "character": 0}, "end": {"line": 4, "character": 0}},
            "newText": "fn main() {\n    print(1) // expect: 1\n}\n"
        }]),
        "{response}"
    );

    // Nothing to do if it's formatted already.
    open(&mut c, "fn main() { print(1) }\n");
    assert_eq!(format(&mut c, 3)["result"], json!([]));

    // Nothing to do if it doesn't parse: the diagnostic says why.
    open(&mut c, "fn main() {");
    let response = format(&mut c, 4);
    assert_eq!(response["result"], Value::Null, "{response}");
    assert!(response.get("error").is_none(), "{response}");

    stop(child, c);
}
