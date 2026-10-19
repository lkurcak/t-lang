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
    for cap in [
        "referencesProvider",
        "documentHighlightProvider",
        "typeDefinitionProvider",
        "documentSymbolProvider",
    ] {
        assert_eq!(caps[cap], true, "{cap}: {init}");
    }
    assert_eq!(caps["renameProvider"]["prepareProvider"], true, "{init}");
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
fn borrow_errors_point_at_the_borrow() {
    let (child, mut c) = start();
    let uri = "file:///tmp/borrow.t";
    let src = "fn main() {\n    let mut x = 1;\n    let r = &x;\n    x = 2;\n    print(*r);\n}\n";
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": uri, "languageId": "t", "version": 1, "text": src}
        }}),
    );
    let diags = c.diagnostics();
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(
        diags[0]["message"],
        "type error: cannot assign to `x` while `x` is borrowed"
    );
    assert_eq!(
        diags[0]["relatedInformation"],
        json!([{
            "location": {
                "uri": uri,
                "range": {"start": {"line": 2, "character": 12}, "end": {"line": 2, "character": 14}}
            },
            "message": "`x` is borrowed here"
        }])
    );
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
fn hovers_and_hints_show_opaque_types_as_written() {
    let (child, mut c) = start();
    let uri = "file:///tmp/opaque.t";
    let src = "fn adder(n: int) -> impl Fn(int) -> int + Discard { move |x| x + n }\nfn main() {\n    let f = adder(1);\n    print(f(2));\n}\n";
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": uri, "languageId": "t", "version": 1, "text": src}
        }}),
    );
    assert_eq!(c.diagnostics(), Vec::<Value>::new());
    let at = |line: u32, character: u32| json!({"textDocument": {"uri": uri}, "position": {"line": line, "character": character}});

    // Callers see what the signature says, not the closure behind it.
    c.send(json!({"jsonrpc": "2.0", "id": 2, "method": "textDocument/hover", "params": at(3, 10)}));
    let hover = c.response(2)["result"].clone();
    assert_eq!(
        hover["contents"]["value"], "```t\nf: impl Fn(int) -> int + Discard\n```",
        "{hover}"
    );
    let whole = json!({"start": {"line": 0, "character": 0}, "end": {"line": 5, "character": 0}});
    c.send(
        json!({"jsonrpc": "2.0", "id": 3, "method": "textDocument/inlayHint", "params": {
            "textDocument": {"uri": uri}, "range": whole
        }}),
    );
    let hints = c.response(3)["result"].clone();
    let labels: Vec<Value> = hints
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h["label"].clone())
        .collect();
    assert!(
        labels.contains(&json!(": impl Fn(int) -> int + Discard")),
        "{hints}"
    );

    stop(child, c);
}

#[test]
fn hovers_show_doc_comments() {
    let (child, mut c) = start();
    let uri = "file:///tmp/docs.t";
    let src = "\
/// A point.
///
///     Indented, so code.
struct P {
    /// Across.
    x: int,
}
enum E {
    /// The only one.
    A(int),
}
/// Reads `x`.
fn get(p: P) -> int { p.x }
fn main() {
    print(get(P { x: 1 }));
    match E::A(2) {
        E::A(n) => print(n),
    }
}
impl Discard for P {}
impl Discard for E {}
";
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": uri, "languageId": "t", "version": 1, "text": src}
        }}),
    );
    assert_eq!(c.diagnostics(), Vec::<Value>::new());
    let mut id = 1;
    let mut hover = |c: &mut Client, needle: &str, nth: usize| {
        // The `nth` place `needle` is in the source.
        let offset = src.match_indices(needle).nth(nth).unwrap().0;
        let line = src[..offset].matches('\n').count();
        let character = offset - src[..offset].rfind('\n').map_or(0, |i| i + 1);
        id += 1;
        c.send(
            json!({"jsonrpc": "2.0", "id": id, "method": "textDocument/hover", "params": {
                "textDocument": {"uri": uri}, "position": {"line": line, "character": character}
            }}),
        );
        c.response(id)["result"]["contents"]["value"].clone()
    };

    // The signature, then the doc as Markdown, without the indentation
    // all its lines share.
    let get = "```t\nfn get: fn(P) -> int\n```\n\n---\n\nReads `x`.";
    assert_eq!(hover(&mut c, "get", 0), get, "the declaration");
    assert_eq!(hover(&mut c, "get", 1), get, "a use");
    let p = "```t\nstruct P\n```\n\n---\n\nA point.\n\n    Indented, so code.";
    assert_eq!(hover(&mut c, "P {", 0), p);
    assert_eq!(hover(&mut c, "P {", 1), p, "a struct literal");
    assert_eq!(hover(&mut c, "P)", 0), p, "a type in a signature");
    let x = "```t\nfield x: int\n```\n\n---\n\nAcross.";
    assert_eq!(hover(&mut c, "x: int", 0), x);
    assert_eq!(hover(&mut c, "x }", 0), x, "a field read");
    let a = "```t\nE::A: fn(int) -> E\n```\n\n---\n\nThe only one.";
    assert_eq!(hover(&mut c, "A(", 1), a, "a variant built");
    assert_eq!(hover(&mut c, "A(", 2), a, "a variant matched");
    // Nothing documents `E` or `main`: just the signature.
    assert_eq!(hover(&mut c, "E {", 0), "```t\nenum E\n```");
    assert_eq!(hover(&mut c, "main", 0), "```t\nfn main: fn() -> ()\n```");

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

/// Hover and go to definition find the function a name refers to, when
/// nested functions share a name with each other and with a top-level one.
#[test]
fn finds_nested_functions_that_share_a_name() {
    let (child, mut c) = start();
    let uri = "file:///tmp/nested.t";
    let src = "\
/// The top-level one.
fn helper() -> int { 1 }
fn main() {
    print(helper());
    {
        print_bool(helper(2));
        /// The nested one.
        fn helper(x: int) -> bool { x > 1 }
    }
    print(other());
}
fn other() -> int {
    fn helper() -> int { 3 }
    helper()
}
";
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": uri, "languageId": "t", "version": 1, "text": src}
        }}),
    );
    assert_eq!(c.diagnostics(), Vec::<Value>::new());
    // The line and column of the `nth` place `needle` is in the source.
    let at = |needle: &str, nth: usize| {
        let offset = src.match_indices(needle).nth(nth).unwrap().0;
        let line = src[..offset].matches('\n').count();
        let character = offset - src[..offset].rfind('\n').map_or(0, |i| i + 1);
        json!({"line": line, "character": character})
    };
    // Asks for `method` at the `nth` `helper`.
    let mut id = 1;
    let mut ask = |c: &mut Client, method: &str, nth: usize| {
        id += 1;
        c.send(
            json!({"jsonrpc": "2.0", "id": id, "method": method, "params": {
                "textDocument": {"uri": uri}, "position": at("helper", nth)
            }}),
        );
        c.response(id)["result"].clone()
    };
    let (hover, def) = ("textDocument/hover", "textDocument/definition");

    // `helper`s in order: 0 the top-level one, 1 its use in `main`, 2 the
    // nested one's use before it, 3 the nested one, 4 `other`'s, 5 its use.
    let top = "```t\nfn helper: fn() -> int\n```\n\n---\n\nThe top-level one.";
    let nested = "```t\nfn helper: fn(int) -> bool\n```\n\n---\n\nThe nested one.";
    let others = "```t\nfn helper: fn() -> int\n```";
    assert_eq!(ask(&mut c, hover, 1)["contents"]["value"], top);
    assert_eq!(ask(&mut c, def, 1)["range"]["start"], at("helper", 0));
    let used_before = ask(&mut c, hover, 2);
    assert_eq!(
        used_before["contents"]["value"], nested,
        "used before it's declared"
    );
    assert_eq!(ask(&mut c, def, 2)["range"]["start"], at("helper", 3));
    assert_eq!(
        ask(&mut c, hover, 3)["contents"]["value"],
        nested,
        "the declaration"
    );
    assert_eq!(ask(&mut c, hover, 5)["contents"]["value"], others);
    assert_eq!(ask(&mut c, def, 5)["range"]["start"], at("helper", 4));

    stop(child, c);
}

/// Hover and go to definition follow a local type to the one it refers
/// to, when several share a name: in a type, a struct literal, a pattern
/// and a field.
#[test]
fn finds_local_types_that_share_a_name() {
    let (child, mut c) = start();
    let uri = "file:///tmp/local.t";
    let src = "\
/// The top-level one.
struct P { x: int }
fn main() {
    let a: P = P { x: 1 };
    print(a.x);
    {
        let P { y } = P { y: true };
        print_bool(y);
        /// The local one.
        struct P { y: bool }
    }
    print(other().x);
}
fn other() -> P {
    let local = P { z: 3 };
    #[derive(Discard)] struct P { z: int }
    top(local.z)
}
fn top(x: int) -> P { P { x } }
impl Discard for P {}
";
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": uri, "languageId": "t", "version": 1, "text": src}
        }}),
    );
    assert_eq!(c.diagnostics(), Vec::<Value>::new());
    // The line and column of the `nth` place `needle` is in the source.
    let at = |needle: &str, nth: usize| {
        let offset = src.match_indices(needle).nth(nth).unwrap().0;
        let line = src[..offset].matches('\n').count();
        let character = offset - src[..offset].rfind('\n').map_or(0, |i| i + 1);
        json!({"line": line, "character": character})
    };
    let mut id = 1;
    let mut ask = |c: &mut Client, method: &str, needle: &str, nth: usize| {
        id += 1;
        c.send(
            json!({"jsonrpc": "2.0", "id": id, "method": method, "params": {
                "textDocument": {"uri": uri}, "position": at(needle, nth)
            }}),
        );
        c.response(id)["result"].clone()
    };
    let (hover, def) = ("textDocument/hover", "textDocument/definition");

    // `P`s in order: 0 the top-level one, 1 and 2 its uses in `main`, 3
    // and 4 the local one's pattern and literal, before it, 5 the local
    // one, 6 `other`'s result, which is the top-level one, 7 a literal of
    // `other`'s own, before it, 8 that, and 9 and 10 the top-level one in
    // `top`.
    let top = "```t\nstruct P\n```\n\n---\n\nThe top-level one.";
    let local = "```t\nstruct P\n```\n\n---\n\nThe local one.";
    assert_eq!(ask(&mut c, hover, "P", 1)["contents"]["value"], top);
    assert_eq!(ask(&mut c, def, "P", 2)["range"]["start"], at("P", 0));
    assert_eq!(ask(&mut c, hover, "P", 3)["contents"]["value"], local);
    assert_eq!(ask(&mut c, def, "P", 3)["range"]["start"], at("P", 5));
    assert_eq!(ask(&mut c, def, "P", 4)["range"]["start"], at("P", 5));
    assert_eq!(
        ask(&mut c, def, "P", 6)["range"]["start"],
        at("P", 0),
        "a signature is outside the body"
    );
    assert_eq!(ask(&mut c, def, "P", 7)["range"]["start"], at("P", 8));
    assert_eq!(ask(&mut c, def, "P", 10)["range"]["start"], at("P", 0));
    // A field of `other`'s, by the type of what it's read from.
    assert_eq!(ask(&mut c, def, "z", 2)["range"]["start"], at("z", 1));
    assert_eq!(ask(&mut c, def, "z", 0)["range"]["start"], at("z", 1));

    stop(child, c);
}

/// A file open in `t lsp`, for asking about places in it.
struct Doc {
    uri: &'static str,
    src: &'static str,
    id: i64,
}

impl Doc {
    /// Opens `src` and checks it has the diagnostics `want` (by message).
    fn open(c: &mut Client, uri: &'static str, src: &'static str) -> (Doc, Vec<Value>) {
        c.send(
            json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
                "textDocument": {"uri": uri, "languageId": "t", "version": 1, "text": src}
            }}),
        );
        let diags = c.diagnostics();
        (Doc { uri, src, id: 1 }, diags)
    }

    /// The position of the `nth` place `needle` is in the source.
    fn at(&self, needle: &str, nth: usize) -> Value {
        let offset = self
            .src
            .match_indices(needle)
            .nth(nth)
            .unwrap_or_else(|| panic!("no {nth}th {needle:?}"))
            .0;
        let line = self.src[..offset].matches('\n').count();
        let character = offset - self.src[..offset].rfind('\n').map_or(0, |i| i + 1);
        json!({"line": line, "character": character})
    }

    /// The range of the `nth` `needle`.
    fn range(&self, needle: &str, nth: usize) -> Value {
        let start = self.at(needle, nth);
        let end = json!({
            "line": start["line"],
            "character": start["character"].as_u64().unwrap() + needle.len() as u64
        });
        json!({"start": start, "end": end})
    }

    /// The response to `method` at the `nth` `needle`, with `params` added.
    fn ask(
        &mut self,
        c: &mut Client,
        method: &str,
        needle: &str,
        nth: usize,
        params: Value,
    ) -> Value {
        self.id += 1;
        let mut p = json!({"textDocument": {"uri": self.uri}, "position": self.at(needle, nth)});
        for (k, v) in params.as_object().unwrap() {
            p[k] = v.clone();
        }
        c.send(json!({"jsonrpc": "2.0", "id": self.id, "method": method, "params": p}));
        c.response(self.id)
    }

    /// The ranges `textDocument/references` gives at the `nth` `needle`.
    fn references(&mut self, c: &mut Client, needle: &str, nth: usize, decl: bool) -> Vec<Value> {
        let context = json!({"context": {"includeDeclaration": decl}});
        let r = self.ask(c, "textDocument/references", needle, nth, context);
        let refs = r["result"].as_array().unwrap_or_else(|| panic!("{r}"));
        refs.iter()
            .map(|l| {
                assert_eq!(l["uri"], self.uri);
                l["range"].clone()
            })
            .collect()
    }

    /// The source after renaming at the `nth` `needle` to `new`, or the
    /// error message.
    fn rename(
        &mut self,
        c: &mut Client,
        needle: &str,
        nth: usize,
        new: &str,
    ) -> Result<String, String> {
        let r = self.ask(
            c,
            "textDocument/rename",
            needle,
            nth,
            json!({"newName": new}),
        );
        if let Some(e) = r.get("error") {
            return Err(e["message"].as_str().unwrap().to_string());
        }
        let edits = r["result"]["changes"][self.uri]
            .as_array()
            .unwrap_or_else(|| panic!("{r}"));
        let offset = |p: &Value| {
            let line = p["line"].as_u64().unwrap() as usize;
            let start: usize = self
                .src
                .split_inclusive('\n')
                .take(line)
                .map(str::len)
                .sum();
            start + p["character"].as_u64().unwrap() as usize
        };
        let mut edits: Vec<(usize, usize, &str)> = edits
            .iter()
            .map(|e| {
                let r = &e["range"];
                (
                    offset(&r["start"]),
                    offset(&r["end"]),
                    e["newText"].as_str().unwrap(),
                )
            })
            .collect();
        edits.sort_by_key(|e| std::cmp::Reverse(e.0));
        let mut out = self.src.to_string();
        for (start, end, text) in edits {
            out.replace_range(start..end, text);
        }
        Ok(out)
    }
}

/// References, highlights and renames follow what names refer to, when
/// nested functions and local types share a name with others.
#[test]
fn references_highlights_and_renames_follow_scopes() {
    let (child, mut c) = start();
    let src = "\
fn helper() -> int { 1 }
fn main() {
    print(helper());
    {
        print_bool(helper(2));
        // helper is nested here.
        fn helper(x: int) -> bool { x > 1 }
    }
    let p = P { x: 1 };
    print(p.x + other());
    {
        let q = P { y: true };
        print_bool(q.y);
        #[derive(Discard)] struct P { y: bool }
    }
}
fn other() -> int {
    fn helper() -> int { 3 }
    helper()
}
struct P { x: int }
impl Discard for P {}
";
    let (mut d, diags) = Doc::open(&mut c, "file:///tmp/refs.t", src);
    assert_eq!(diags, Vec::<Value>::new());

    // `helper`s in order: 0 the top-level one, 1 its use, 2 the nested
    // one's use before it, 3 a comment, 4 the nested one, 5 `other`'s, 6
    // its use.
    let nested = vec![d.range("helper", 2), d.range("helper", 4)];
    assert_eq!(d.references(&mut c, "helper", 2, true), nested);
    assert_eq!(d.references(&mut c, "helper", 4, true), nested);
    assert_eq!(d.references(&mut c, "helper", 4, false), nested[..1]);
    let top = vec![d.range("helper", 0), d.range("helper", 1)];
    assert_eq!(d.references(&mut c, "helper", 1, true), top);

    // The declaration is written, the uses read.
    let h = d.ask(
        &mut c,
        "textDocument/documentHighlight",
        "helper",
        6,
        json!({}),
    );
    assert_eq!(
        h["result"],
        json!([
            {"range": d.range("helper", 5), "kind": 3},
            {"range": d.range("helper", 6), "kind": 2},
        ])
    );

    // Renaming the nested one renames it and its use, not the comment.
    let renamed = d.rename(&mut c, "helper", 2, "big").unwrap();
    assert_eq!(
        renamed,
        src.replace("helper(2)", "big(2)")
            .replace("fn helper(x", "fn big(x")
    );

    // `P`s in order: 0 the top-level one's literal, 1 the local one's
    // literal, 2 the local one, 3 the top-level one.
    assert_eq!(
        d.references(&mut c, "P", 1, true),
        vec![d.range("P", 1), d.range("P", 2)]
    );
    let renamed = d.rename(&mut c, "P", 3, "Point").unwrap();
    assert_eq!(
        renamed,
        src.replace("P { x", "Point { x")
            .replace("for P {}", "for Point {}")
    );
    // A field of the local one.
    assert_eq!(
        d.references(&mut c, "y", 2, true),
        vec![d.range("y", 0), d.range("y", 1), d.range("y", 2)]
    );

    stop(child, c);
}

/// Renaming either side of a shorthand `P { x }` spells out the other.
#[test]
fn renames_shorthand_fields_and_variables() {
    let (child, mut c) = start();
    let src = "\
struct P { x: int }
fn main() {
    let x = 1;
    let p = P { x };
    let P { x } = p;
    print(x);
}
";
    let (mut d, diags) = Doc::open(&mut c, "file:///tmp/shorthand.t", src);
    assert_eq!(diags, Vec::<Value>::new());
    // `x`s in order: 0 the field, 1 the first variable, 2 the literal, 3
    // the pattern, binding the second variable, 4 its use.
    assert_eq!(
        d.rename(&mut c, "x", 1, "a").unwrap(),
        src.replace("let x = 1", "let a = 1")
            .replace("P { x };", "P { x: a };")
    );
    assert_eq!(
        d.rename(&mut c, "x", 2, "a").unwrap(),
        d.rename(&mut c, "x", 1, "a").unwrap(),
        "from the shorthand, the variable"
    );
    assert_eq!(
        d.rename(&mut c, "x", 3, "b").unwrap(),
        src.replace("P { x } =", "P { x: b } =")
            .replace("print(x)", "print(b)")
    );
    assert_eq!(
        d.rename(&mut c, "x", 0, "y").unwrap(),
        src.replace("{ x: int", "{ y: int")
            .replace("P { x };", "P { y: x };")
            .replace("P { x } =", "P { y: x } =")
    );

    stop(child, c);
}

/// Fields, variants and type parameters rename too, a type parameter only
/// in what declares it.
#[test]
fn renames_fields_variants_and_type_parameters() {
    let (child, mut c) = start();
    let src = "\
enum Shape<T> {
    Circle(T),
    Empty,
}
struct Wrap<T> { inner: T }
fn area<T: Discard>(s: Shape<T>, w: Wrap<T>) -> T {
    match s {
        Shape::Circle(r) => r,
        Shape::Empty => w.inner,
    }
}
fn main() {
    print(area(Shape::Circle(2), Wrap { inner: 1 }));
    print(area(Shape::Empty, Wrap::<int> { inner: 3 }));
}
impl<T: Discard> Discard for Shape<T> {}
impl<T: Discard> Discard for Wrap<T> {}
";
    let (mut d, diags) = Doc::open(&mut c, "file:///tmp/kinds.t", src);
    assert_eq!(diags, Vec::<Value>::new());
    assert_eq!(
        d.rename(&mut c, "Circle", 0, "Round").unwrap(),
        src.replace("Circle", "Round")
    );
    assert_eq!(
        d.rename(&mut c, "inner", 1, "value").unwrap(),
        src.replace("inner", "value")
    );
    // `T`s in order: 0 and 1 `Shape`'s, 2 and 3 `Wrap`'s, then `area`'s.
    assert_eq!(
        d.rename(&mut c, "T", 4, "U").unwrap(),
        src.replace(
            "area<T: Discard>(s: Shape<T>, w: Wrap<T>) -> T",
            "area<U: Discard>(s: Shape<U>, w: Wrap<U>) -> U"
        )
    );
    assert_eq!(
        d.rename(&mut c, "T", 1, "Item").unwrap(),
        src.replace(
            "Shape<T> {\n    Circle(T)",
            "Shape<Item> {\n    Circle(Item)"
        )
    );

    stop(child, c);
}

/// What can't be renamed, and renames that are refused.
#[test]
fn refuses_bad_renames() {
    let (child, mut c) = start();
    let src = "\
struct P { x: int }
fn main() {
    let a = 1;
    let b = 2;
    let p = P { x: 4 };
    fn helper() -> int { c() }
    print(a + b + p.x + helper());
}
fn c() -> int { 3 }
impl Discard for P {}
";
    let (mut d, diags) = Doc::open(&mut c, "file:///tmp/bad.t", src);
    assert_eq!(diags, Vec::<Value>::new());
    let mut prepare = |c: &mut Client, needle: &str, nth: usize| {
        let r = d.ask(c, "textDocument/prepareRename", needle, nth, json!({}));
        match r.get("error") {
            Some(e) => Err(e["message"].clone()),
            None => Ok(r["result"].clone()),
        }
    };
    assert_eq!(
        prepare(&mut c, "print", 0),
        Err(json!("`print` is a built-in function"))
    );
    assert_eq!(
        prepare(&mut c, "int", 0),
        Err(json!("`int` is a built-in type"))
    );
    assert_eq!(
        prepare(&mut c, "let", 0),
        Err(json!("there's nothing here to rename"))
    );
    assert_eq!(
        prepare(&mut c, "1", 0),
        Err(json!("there's nothing here to rename"))
    );
    assert_eq!(
        prepare(&mut c, "main", 0),
        Err(json!(
            "`main` can't be renamed: it's where the program starts"
        ))
    );
    let a = prepare(&mut c, "a", 1);
    assert_eq!(a, Ok(d.range("a", 1)));

    assert_eq!(
        d.rename(&mut c, "a", 1, "1a"),
        Err("`1a` isn't a name".into())
    );
    assert_eq!(
        d.rename(&mut c, "a", 1, "a b"),
        Err("`a b` isn't a name".into())
    );
    assert_eq!(
        d.rename(&mut c, "a", 1, "match"),
        Err("`match` is a keyword".into())
    );
    assert_eq!(
        d.rename(&mut c, "a", 1, "_"),
        Err("`_` can't be a name".into())
    );
    assert_eq!(
        d.rename(&mut c, "P", 0, "int"),
        Err("`int` is a built-in type".into())
    );
    // A variable shadowed by another of the new name.
    assert_eq!(
        d.rename(&mut c, "a", 1, "b"),
        Err(
            "can't rename `a` to `b`: `b` on line 7 would refer to the variable `b` on line 4 \
             instead"
                .into()
        )
    );
    // A function that would be shadowed by the renamed variable.
    assert_eq!(
        d.rename(&mut c, "a", 1, "print"),
        Err(
            "can't rename `a` to `print`: line 7 would be an error: `int` is not a function".into()
        )
    );
    // A nested `fn` would then capture it.
    assert_eq!(
        d.rename(&mut c, "a", 1, "c"),
        Err(
            "can't rename `a` to `c`: line 6 would be an error: can't capture `c` in a nested \
             `fn`; use a closure"
                .into()
        )
    );
    // A use of the renamed one would refer to another.
    assert_eq!(
        d.rename(&mut c, "c() -> int { 3", 0, "helper"),
        Err(
            "can't rename `c` to `helper`: `helper` on line 6 would refer to the function \
             `helper` on line 6 instead"
                .into()
        )
    );
    // The renamed one would shadow another.
    assert_eq!(
        d.rename(&mut c, "helper", 0, "c"),
        Err(
            "can't rename `helper` to `c`: it would shadow the function `c` on line 9, used \
             on line 6"
                .into()
        )
    );
    // Two of one name.
    assert_eq!(
        d.rename(&mut c, "c() -> int { 3", 0, "main"),
        Err(
            "can't rename `c` to `main`: line 9 would be an error: `main` is already defined"
                .into()
        )
    );

    stop(child, c);
}

/// Go to type definition finds every struct and enum in a type.
#[test]
fn goes_to_type_definitions() {
    let (child, mut c) = start();
    let src = "\
enum Option<T> {
    Some(T),
    None,
}
struct P { x: int }
fn get(o: Option<P>) -> int {
    match o {
        Option::Some(p) => p.x,
        Option::None => 0,
    }
}
fn id<T>(t: T) -> T { t }
fn main() {
    let o = Option::Some(P { x: 1 });
    print(get(o));
    print(get(Option::None));
    print(id(2));
}
impl<T: Discard> Discard for Option<T> {}
impl Discard for P {}
";
    let (mut d, diags) = Doc::open(&mut c, "file:///tmp/typedef.t", src);
    assert_eq!(diags, Vec::<Value>::new());
    let mut ty = |c: &mut Client, needle: &str, nth: usize| {
        let r = d.ask(c, "textDocument/typeDefinition", needle, nth, json!({}));
        let r = r["result"].clone();
        match r.as_array() {
            Some(locations) => locations
                .iter()
                .map(|l| l["range"]["start"].clone())
                .collect(),
            None => {
                assert_eq!(r, Value::Null);
                Vec::new()
            }
        }
    };
    let (option, p) = (
        json!({"line": 0, "character": 5}),
        json!({"line": 4, "character": 7}),
    );
    // `o`s: 0 the parameter, 1 its use, 2 the variable, 3 its use.
    assert_eq!(ty(&mut c, "o:", 0), vec![option.clone(), p.clone()]);
    assert_eq!(ty(&mut c, "o))", 0), vec![option.clone(), p.clone()]);
    assert_eq!(ty(&mut c, "p)", 0), vec![p.clone()], "a pattern binding");
    assert_eq!(ty(&mut c, "get", 1), vec![option, p], "a function");
    assert_eq!(ty(&mut c, "x", 1), Vec::<Value>::new(), "an `int` field");
    assert_eq!(
        ty(&mut c, "t }", 0),
        Vec::<Value>::new(),
        "a type parameter"
    );
    assert_eq!(ty(&mut c, "main", 0), Vec::<Value>::new(), "`fn() -> ()`");

    stop(child, c);
}

/// The outline: items, with what's in them under them, from the parse
/// alone.
#[test]
fn outlines_documents() {
    let (child, mut c) = start();
    let mut id = 1;
    let mut outline = |c: &mut Client, uri: &str| {
        id += 1;
        c.send(
            json!({"jsonrpc": "2.0", "id": id, "method": "textDocument/documentSymbol", "params": {
                "textDocument": {"uri": uri}
            }}),
        );
        c.response(id)["result"].clone()
    };
    // Name, kind, detail and children, for comparing.
    fn tree(symbols: &Value) -> Vec<Value> {
        symbols
            .as_array()
            .unwrap()
            .iter()
            .map(|s| json!([s["name"], s["kind"], s["detail"], tree(&s["children"])]))
            .collect()
    }

    let src = "\
struct Pair<A, B> {
    first: A,
    second: B,
}
fn main() {
    print(helper(1));
    fn helper(x: int) -> int {
        enum E { A(int), B }
        fn inner() {}
        x
    }
    struct P { x: int }
}
enum Option<T> {
    Some(T),
    None,
}
";
    let (d, diags) = Doc::open(&mut c, "file:///tmp/outline.t", src);
    assert!(!diags.is_empty(), "dead code");
    let symbols = outline(&mut c, d.uri);
    assert_eq!(
        tree(&symbols),
        vec![
            json!([
                "Pair",
                23,
                "struct Pair<A, B>",
                [["first", 8, "A", []], ["second", 8, "B", []],]
            ]),
            json!([
                "main",
                12,
                "fn() -> ()",
                [
                    [
                        "helper",
                        12,
                        "fn(int) -> int",
                        [
                            [
                                "E",
                                10,
                                "enum E",
                                [["A", 22, "fn(int) -> E", []], ["B", 22, "E", []],]
                            ],
                            ["inner", 12, "fn() -> ()", []],
                        ]
                    ],
                    ["P", 23, "struct P", [["x", 8, "int", []]]],
                ]
            ]),
            json!([
                "Option",
                10,
                "enum Option<T>",
                [
                    ["Some", 22, "fn(T) -> Option<T>", []],
                    ["None", 22, "Option<T>", []],
                ]
            ]),
        ]
    );
    // The whole of `Pair`, and its name.
    assert_eq!(
        symbols[0]["range"],
        json!({"start": {"line": 0, "character": 0}, "end": {"line": 3, "character": 1}})
    );
    assert_eq!(symbols[0]["selectionRange"], d.range("Pair", 0));
    assert_eq!(
        symbols[2]["children"][0]["range"],
        json!({"start": {"line": 14, "character": 4}, "end": {"line": 14, "character": 11}})
    );

    // With a type error, from the parse.
    let (d, diags) = Doc::open(
        &mut c,
        "file:///tmp/outline_err.t",
        "fn main() { let x: int = true; }\nstruct S {}\n",
    );
    assert_eq!(diags.len(), 1);
    assert_eq!(
        tree(&outline(&mut c, d.uri)),
        vec![
            json!(["main", 12, "fn() -> ()", []]),
            json!(["S", 23, "struct S", []]),
        ]
    );

    stop(child, c);
}

/// Hover, go to definition, rename and the outline know traits, impls and
/// methods. A method of an impl of a trait is renamed with the trait's.
#[test]
fn traits_and_methods() {
    let (child, mut c) = start();
    let src = "\
trait Show {
    fn show(&self) -> int;
}
struct P { x: int }
impl P {
    fn get(&self) -> int { self.x }
}
impl Show for P {
    fn show(&self) -> int { self.get() }
}
fn twice<T: Show + Discard>(t: T) -> int { t.show() * 2 }
fn main() {
    let p = P { x: 1 };
    print(twice(p));
    print(P { x: 2 }.show());
}
impl Discard for P {}
";
    let (mut d, diags) = Doc::open(&mut c, "file:///tmp/traits.t", src);
    assert_eq!(diags, Vec::<Value>::new());
    let hover = d.ask(&mut c, "textDocument/hover", "Show +", 0, json!({}));
    assert_eq!(
        hover["result"]["contents"]["value"],
        "```t\ntrait Show\n```"
    );
    let hover = d.ask(&mut c, "textDocument/hover", "get()", 0, json!({}));
    assert_eq!(
        hover["result"]["contents"]["value"],
        "```t\nfn get: fn(&P) -> int\n```"
    );

    // A call of a trait's method goes to the trait's, and `get` to the impl's.
    let def = d.ask(&mut c, "textDocument/definition", "show()", 1, json!({}));
    assert_eq!(def["result"]["range"], d.range("show", 0));
    let def = d.ask(&mut c, "textDocument/definition", "get()", 0, json!({}));
    assert_eq!(def["result"]["range"], d.range("get", 0));
    let def = d.ask(&mut c, "textDocument/definition", "Show for", 0, json!({}));
    assert_eq!(def["result"]["range"], d.range("Show", 0));

    assert_eq!(
        d.rename(&mut c, "show", 2, "display").unwrap(),
        src.replace("show", "display")
    );
    assert_eq!(
        d.rename(&mut c, "Show", 2, "Display").unwrap(),
        src.replace("Show", "Display")
    );
    assert_eq!(
        d.rename(&mut c, "get", 0, "value").unwrap(),
        src.replace("get", "value")
    );

    d.id += 1;
    c.send(
        json!({"jsonrpc": "2.0", "id": d.id, "method": "textDocument/documentSymbol", "params": {
            "textDocument": {"uri": d.uri}
        }}),
    );
    let symbols = c.response(d.id)["result"].clone();
    let names: Vec<Value> = symbols
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            let children: Vec<Value> = s["children"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| c["name"].clone())
                .collect();
            json!([s["name"], s["kind"], children])
        })
        .collect();
    assert_eq!(
        names,
        vec![
            json!(["Show", 11, ["show"]]),
            json!(["P", 23, ["x"]]),
            json!(["impl P", 19, ["get"]]),
            json!(["impl Show for P", 19, ["show"]]),
            json!(["twice", 12, []]),
            json!(["main", 12, []]),
            json!(["impl Discard for P", 19, []]),
        ]
    );

    stop(child, c);
}

/// Hover, go to definition, rename and the outline work through paths and
/// `use`s. A name brought in with `as` is another name, which renaming the
/// item leaves alone.
#[test]
fn modules() {
    let (child, mut c) = start();
    let src = "\
mod shapes {
    pub trait Area {
        fn area(&self) -> int;
    }
    #[derive(Discard)]
    pub struct Sq { pub side: int }
    pub fn sq(side: int) -> Sq { Sq { side } }
    pub mod more {
        pub fn twice(n: int) -> int { n * 2 }
    }
}
use shapes::{Area, Sq, more::twice as double};
impl Area for Sq {
    fn area(&self) -> int { self.side * self.side }
}
fn main() {
    let s: Sq = shapes::sq(3);
    print(s.area() + double(1) + shapes::more::twice(2));
}
";
    let (mut d, diags) = Doc::open(&mut c, "file:///tmp/modules.t", src);
    assert_eq!(diags, Vec::<Value>::new());
    let hover = d.ask(&mut c, "textDocument/hover", "sq(3)", 0, json!({}));
    assert_eq!(
        hover["result"]["contents"]["value"],
        "```t\nfn sq: fn(int) -> Sq\n```"
    );
    let def = d.ask(&mut c, "textDocument/definition", "sq(3)", 0, json!({}));
    assert_eq!(def["result"]["range"], d.range("sq", 0));
    let def = d.ask(&mut c, "textDocument/definition", "double(1)", 0, json!({}));
    assert_eq!(def["result"]["range"], d.range("twice", 0));
    let def = d.ask(&mut c, "textDocument/definition", "Sq, more", 0, json!({}));
    assert_eq!(def["result"]["range"], d.range("Sq", 0));

    assert_eq!(
        d.rename(&mut c, "Sq", 0, "Square").unwrap(),
        src.replace("Sq", "Square")
    );
    assert_eq!(
        d.rename(&mut c, "twice", 0, "dup").unwrap(),
        src.replace("twice", "dup")
    );
    assert_eq!(
        d.rename(&mut c, "Area", 0, "Size").unwrap(),
        src.replace("Area", "Size")
    );

    d.id += 1;
    c.send(
        json!({"jsonrpc": "2.0", "id": d.id, "method": "textDocument/documentSymbol", "params": {
            "textDocument": {"uri": d.uri}
        }}),
    );
    let symbols = c.response(d.id)["result"].clone();
    let names: Vec<Value> = symbols
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            let children: Vec<Value> = s["children"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| c["name"].clone())
                .collect();
            json!([s["name"], s["kind"], children])
        })
        .collect();
    assert_eq!(
        names,
        vec![
            json!(["shapes", 2, ["Area", "Sq", "sq", "more"]]),
            json!(["impl Area for Sq", 19, ["area"]]),
            json!(["main", 12, []]),
        ]
    );

    stop(child, c);
}
