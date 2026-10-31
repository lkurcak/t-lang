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

    /// Collects the latest diagnostics for each file until a following
    /// request is answered. A missing publication is a failure, not a hang.
    fn diagnostics_before_response(&mut self, id: i64) -> std::collections::HashMap<String, Value> {
        let mut out = std::collections::HashMap::new();
        loop {
            let msg = self.recv();
            if msg["id"] == id {
                return out;
            }
            if msg["method"] == "textDocument/publishDiagnostics" {
                let params = &msg["params"];
                out.insert(params["uri"].as_str().unwrap().to_string(), params.clone());
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
    start_command(Command::new(env!("CARGO_BIN_EXE_t")))
}

fn start_command(mut command: Command) -> (std::process::Child, Client) {
    let mut child = command
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
        json!(["quickfix", "refactor.rewrite"]),
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
fn failed_proofs_explain_conditions_and_lost_facts() {
    let uri = "file:///tmp/refinement-diagnostic.t";
    for (src, condition, known, invalidation) in [
        (
            "fn inc(n: i64) -> i64 with {} { n + 1 }\nfn main() { let _ = inc(0); }",
            "n <= 9223372036854775806",
            "n <= 9223372036854775807",
            None,
        ),
        (
            "fn at(s: &[i64], i: usize) -> i64 { s[i] }\nfn main() { let a: [i64; 1] = [1]; let _ = at(a.as_slice(), 0); }",
            "required condition: i < s.len()",
            "0 <= i",
            None,
        ),
        (
            include_str!("t/err_soundness_measure_pop.t"),
            "required condition: 0 < l.len()",
            "0 <= l.len()",
            Some(9),
        ),
        (
            include_str!("t/err_refine_solver_coefficient_min.t"),
            "required condition: 0 <= x",
            "solver's checked integer arithmetic range",
            None,
        ),
    ] {
        let (child, mut c) = start();
        c.send(
            json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
                "textDocument": {"uri": uri, "languageId": "t", "version": 1, "text": src}
            }}),
        );
        let diagnostics = c.diagnostics();
        let errors: Vec<_> = diagnostics.iter().filter(|d| d["severity"] == 1).collect();
        assert_eq!(errors.len(), 1, "{diagnostics:?}");
        let d = errors[0];
        let message = d["message"].as_str().unwrap();
        assert!(message.contains(condition), "{d}");
        assert!(message.contains(known), "{d}");
        if let Some(line) = invalidation {
            let related = d["relatedInformation"].as_array().unwrap();
            assert_eq!(related.len(), 1, "{d}");
            assert_eq!(related[0]["location"]["uri"], uri);
            assert_eq!(related[0]["location"]["range"]["start"]["line"], line);
            assert!(
                related[0]["message"]
                    .as_str()
                    .unwrap()
                    .contains("mutable lending invalidated"),
                "{d}"
            );
        }
        stop(child, c);
    }
}

#[test]
fn loop_proof_failures_link_to_the_loop_and_the_operation_that_may_change_it() {
    for (source, operation, cause, header) in [
        (
            include_str!("t/err_soundness_refine_place_loop.t"),
            "p.x = 0",
            "assignment",
            "while ",
        ),
        (
            include_str!("t/err_soundness_refine_place_short_loop.t"),
            "zero(&mut p)",
            "mutable lending",
            "while ",
        ),
        (
            include_str!("t/err_soundness_measure_loop.t"),
            "s = core::slice::empty()",
            "assignment",
            "while ",
        ),
        (
            include_str!("t/err_refine_loop_for_diagnostic.t"),
            "d = 0",
            "assignment",
            "for ",
        ),
    ] {
        let (child, mut c) = start();
        let (doc, diagnostics) = Doc::open(&mut c, "file:///tmp/loop-proof.t", source);
        let errors: Vec<_> = diagnostics.iter().filter(|d| d["severity"] == 1).collect();
        assert_eq!(errors.len(), 1, "{diagnostics:?}");
        let d = errors[0];
        assert!(
            d["message"]
                .as_str()
                .unwrap()
                .contains("required condition:"),
            "{d}"
        );
        let related = d["relatedInformation"].as_array().unwrap();
        assert_eq!(related.len(), 2, "{d}");
        let change = related
            .iter()
            .find(|note| {
                note["message"]
                    .as_str()
                    .unwrap()
                    .contains("in this loop may change")
            })
            .unwrap();
        assert!(change["message"].as_str().unwrap().contains(cause), "{d}");
        assert_eq!(change["location"]["uri"], doc.uri);
        assert_eq!(change["location"]["range"]["start"], doc.at(operation, 0));
        let loop_note = related
            .iter()
            .find(|note| {
                note["message"]
                    .as_str()
                    .unwrap()
                    .contains("not assumed for every iteration")
            })
            .unwrap();
        assert_eq!(loop_note["location"]["uri"], doc.uri);
        assert_eq!(loop_note["location"]["range"]["start"], doc.at(header, 0));
        stop(child, c);
    }
}

#[test]
fn reports_and_clears_diagnostics() {
    let (child, mut c) = start();
    let uri = "file:///tmp/test.t";

    // Open a file with a type error on line 2 (0-based line 1).
    let bad = "fn main() {\n    let x: i64 = true;\n}\n";
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": uri, "languageId": "t", "version": 1, "text": bad}
        }}),
    );
    let diags = c.diagnostics();
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(
        diags[0]["message"],
        "type error: expected `i64`, found `bool`"
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
    let good = "fn main() {\n    let _x: i64 = 1;\n}\n";
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
    let src = "fn main() {\n    let mut x: i64 = 1;\n    let r = &x;\n    x = 2;\n    print(*r);\n}\nfn print<T: Int>(_x: T) {}\n";
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
    let src = "fn f(b: bool) -> i64 {\n    match b {\n        true => 1,\n        _ => 2,\n        false => 3,\n    }\n}\nfn main() {\n    print(f(true));\n}\nfn print<T: Int>(_x: T) {}\n";
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
    let src = "fn id<T>(x: T) -> T { x }\nfn main() {\n    let n = id(1i64);\n    print(n);\n    print(id::<i64>(2));\n}\nfn print(_x: i64) {}\n";
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": uri, "languageId": "t", "version": 1, "text": src}
        }}),
    );
    assert_eq!(c.diagnostics(), Vec::<Value>::new());
    let at = |line: u32, character: u32| json!({"textDocument": {"uri": uri}, "position": {"line": line, "character": character}});

    // `id` in `id(1)` is the copy of `id` for `i64`.
    c.send(json!({"jsonrpc": "2.0", "id": 2, "method": "textDocument/hover", "params": at(2, 13)}));
    let hover = c.response(2)["result"].clone();
    assert_eq!(
        hover["contents"]["value"], "```t\nfn id::<i64>: fn(i64) -> i64\n```",
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
    // one written out in `id::<i64>(2)`.
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
            (json!({"line": 2, "character": 9}), json!(": i64")),
            (json!({"line": 2, "character": 14}), json!("::<i64>")),
        ]
    );

    stop(child, c);
}

#[test]
fn hovers_and_hints_show_opaque_types_as_written() {
    let (child, mut c) = start();
    let uri = "file:///tmp/opaque.t";
    let src = "fn adder(n: i64) -> impl Fn(i64) -> i64 with panic + Discard { move |x| x + n }\nfn main() with panic {\n    let f = adder(1);\n    print(f(2));\n}\nfn print<T: Int>(_x: T) {}\n";
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
        hover["contents"]["value"], "```t\nf: impl Fn(i64) -> i64 with panic + Discard\n```",
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
        labels.contains(&json!(": impl Fn(i64) -> i64 with panic + Discard")),
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
#[derive(Discard)] struct P {
    /// Across.
    x: i64,
}
#[derive(Discard)] enum E {
    /// The only one.
    A(i64),
}
/// Reads `x`.
fn get(p: P) -> i64 { p.x }
fn main() {
    print(get(P { x: 1 }));
    match E::A(2) {
        E::A(n) => print(n),
    }
}
\nfn print<T: Int>(_x: T) {}\n";
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
    let get = "```t\nfn get: fn(P) -> i64\n```\n\n---\n\nReads `x`.";
    assert_eq!(hover(&mut c, "get", 0), get, "the declaration");
    assert_eq!(hover(&mut c, "get", 1), get, "a use");
    let p = "```t\nstruct P\n```\n\n---\n\nA point.\n\n    Indented, so code.";
    assert_eq!(hover(&mut c, "P {", 0), p);
    assert_eq!(hover(&mut c, "P {", 1), p, "a struct literal");
    assert_eq!(hover(&mut c, "P)", 0), p, "a type in a signature");
    let x = "```t\nfield x: i64\n```\n\n---\n\nAcross.";
    assert_eq!(hover(&mut c, "x: i64", 0), x);
    assert_eq!(hover(&mut c, "x }", 0), x, "a field read");
    let a = "```t\nE::A: fn(i64) -> E\n```\n\n---\n\nThe only one.";
    assert_eq!(hover(&mut c, "A(", 1), a, "a variant built");
    assert_eq!(hover(&mut c, "A(", 2), a, "a variant matched");
    // Nothing documents `E` or `main`: just the signature.
    assert_eq!(hover(&mut c, "E {", 0), "```t\nenum E\n```");
    assert_eq!(hover(&mut c, "main", 0), "```t\nfn main: fn() -> ()\n```");

    stop(child, c);
}

/// A function whose effects are inferred shows them in hovers, and an
/// editor can write them into its signature.
#[test]
fn hovers_show_and_write_inferred_effects() {
    let (child, mut c) = start();
    let uri = "file:///tmp/inferred.t";
    let src = "fn next(n: i64) -> i64 { n + 1 }\nfn main() {\n    let _x = next(4);\n}\n";
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": uri, "languageId": "t", "version": 1, "text": src}
        }}),
    );
    assert_eq!(c.diagnostics(), Vec::<Value>::new());
    let cursor = json!({"line": 0, "character": 4});
    c.send(
        json!({"jsonrpc": "2.0", "id": 2, "method": "textDocument/hover", "params": {
            "textDocument": {"uri": uri}, "position": cursor
        }}),
    );
    let hover = c.response(2)["result"]["contents"]["value"].clone();
    assert_eq!(
        hover, "```t\nfn next: fn(i64) -> i64 with panic\n```",
        "{hover}"
    );
    c.send(
        json!({"jsonrpc": "2.0", "id": 3, "method": "textDocument/codeAction", "params": {
            "textDocument": {"uri": uri},
            "range": {"start": cursor, "end": cursor},
            "context": {"diagnostics": []}
        }}),
    );
    let actions = c.response(3)["result"].clone();
    assert_eq!(actions.as_array().unwrap().len(), 1, "{actions}");
    let action = &actions[0];
    assert_eq!(action["title"], "write its effects: `with panic`");
    assert_eq!(action["kind"], "refactor.rewrite");
    assert_eq!(
        action["edit"]["changes"][uri],
        json!([{
            "range": {"start": {"line": 0, "character": 22}, "end": {"line": 0, "character": 22}},
            "newText": " with panic"
        }])
    );
    stop(child, c);
}

/// A module's `//!` doc comment shows where it's declared, in a `use`, and
/// at the start of a path; the libraries' modules' too.
#[test]
fn hovers_show_module_docs() {
    let (child, mut c) = start();
    let uri = "file:///tmp/modules.t";
    let src = "\
//! The program.
use alloc::list;
mod shapes {
    //! Shapes.
    pub fn one() -> i64 { 1 }
}
fn main() with alloc {
    let mut l = list::List::new();
    l.push(shapes::one());
    print(l.len());
}
\nfn print<T: Int>(_x: T) {}\n";
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": uri, "languageId": "t", "version": 1, "text": src}
        }}),
    );
    assert_eq!(c.diagnostics(), Vec::<Value>::new());
    let mut id = 1;
    let mut hover = |c: &mut Client, needle: &str, nth: usize| {
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
    let shapes = "```t\nmod crate::shapes\n```\n\n---\n\nShapes.";
    assert_eq!(hover(&mut c, "shapes", 0), shapes, "the declaration");
    assert_eq!(hover(&mut c, "shapes", 1), shapes, "a path");
    let list = hover(&mut c, "list", 0);
    let list = list.as_str().unwrap();
    assert!(
        list.starts_with("```t\nmod alloc::list\n```\n\n---\n\nLists"),
        "{list}"
    );
    assert_eq!(hover(&mut c, "list::", 0).as_str(), Some(list), "a path");
    let alloc = hover(&mut c, "alloc", 0);
    let alloc = alloc.as_str().unwrap();
    assert!(
        alloc.starts_with("```t\nmod alloc\n```\n\n---\n\nThe alloc library"),
        "{alloc}"
    );

    stop(child, c);
}

#[test]
fn hovers_show_the_traits_and_box_of_the_libraries() {
    let (child, mut c) = start();
    let uri = "file:///tmp/library_items.t";
    let src = "\
use alloc::boxed::Box;
fn same<T: Eq>(a: &T, b: &T) -> bool { a == b }
fn main() with alloc {
    let b = Box::new(1i64);
    print_bool(same(&*b, &*b));
}
\nfn print_bool(_b: bool) {}\n";
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": uri, "languageId": "t", "version": 1, "text": src}
        }}),
    );
    assert_eq!(c.diagnostics(), Vec::<Value>::new());
    let mut id = 1;
    let mut ask = |c: &mut Client, method: &str, needle: &str| {
        let offset = src.find(needle).unwrap();
        let line = src[..offset].matches('\n').count();
        let character = offset - src[..offset].rfind('\n').map_or(0, |i| i + 1);
        id += 1;
        c.send(
            json!({"jsonrpc": "2.0", "id": id, "method": method, "params": {
                "textDocument": {"uri": uri}, "position": {"line": line, "character": character}
            }}),
        );
        c.response(id)["result"].clone()
    };
    // Core's `Eq`, in the prelude, with its doc.
    let eq = ask(&mut c, "textDocument/hover", "Eq>");
    let eq = eq["contents"]["value"].as_str().unwrap().to_string();
    assert!(eq.starts_with("```t\ntrait Eq\n```"), "{eq}");
    assert!(eq.contains("What `==` and `!=` compare with"), "{eq}");
    // Alloc's `Box`, imported.
    let boxed = ask(&mut c, "textDocument/hover", "Box::new");
    let boxed = boxed["contents"]["value"].as_str().unwrap().to_string();
    assert!(boxed.contains("A value on the heap"), "{boxed}");

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

/// Formatting keeps how a literal is written and the parentheses that
/// say which of two bitwise operators comes first; hovering over a `!`
/// of an integer shows the integer's type.
#[test]
fn formats_and_hovers_bitwise_operators() {
    let (child, mut c) = start();
    let uri = "file:///tmp/bits.t";
    let src = "fn low(x: u16) -> u8 {   ((x >> 4)  & 0x0F) as u8 | 0b1000_0000 }\nfn main() { let m = !low(0xF0); print(m) }\nfn print<T: Int>(_x: T) {}\n";
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": uri, "languageId": "t", "version": 1, "text": src}
        }}),
    );
    assert_eq!(c.diagnostics(), Vec::<Value>::new());
    c.send(
        json!({"jsonrpc": "2.0", "id": 2, "method": "textDocument/formatting", "params": {
            "textDocument": {"uri": uri},
            "options": {"tabSize": 4, "insertSpaces": true}
        }}),
    );
    let response = c.response(2);
    assert_eq!(
        response["result"][0]["newText"],
        "fn low(x: u16) -> u8 { ((x >> 4) & 0x0F) as u8 | 0b1000_0000 }\nfn main() {\n    let m = !low(0xF0);\n    print(m)\n}\nfn print<T: Int>(_x: T) {}\n",
        "{response}"
    );
    c.send(
        json!({"jsonrpc": "2.0", "id": 3, "method": "textDocument/hover", "params": {
            "textDocument": {"uri": uri}, "position": {"line": 1, "character": 16}
        }}),
    );
    let hover = c.response(3)["result"].clone();
    assert_eq!(hover["contents"]["value"], "```t\nm: u8\n```", "{hover}");
    stop(child, c);
}

/// A measure named in a refinement is a use of it: hover shows it, and
/// go to definition finds it.
#[test]
fn hovers_and_finds_measures_in_refinements() {
    let (child, mut c) = start();
    let uri = "file:///tmp/measure.t";
    let src = "#[derive(Discard)]\nstruct W { n: usize }\nimpl W {\n    #[measure]\n    fn len(&self) -> usize { self.n }\n}\nfn at(w: &W, i: { i: usize | i < w.len() }) -> usize { i }\nfn main() {\n    let w = W { n: 2 };\n    let _x = at(&w, 1);\n}\n";
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": uri, "languageId": "t", "version": 1, "text": src}
        }}),
    );
    assert_eq!(c.diagnostics(), Vec::<Value>::new());
    let at = |line: u32, character: u32| json!({"textDocument": {"uri": uri}, "position": {"line": line, "character": character}});
    c.send(json!({"jsonrpc": "2.0", "id": 2, "method": "textDocument/hover", "params": at(6, 36)}));
    let hover = c.response(2)["result"].clone();
    assert_eq!(
        hover["contents"]["value"], "```t\nfn len: fn(&W) -> usize\n```",
        "{hover}"
    );
    c.send(json!({"jsonrpc": "2.0", "id": 3, "method": "textDocument/definition", "params": at(6, 36)}));
    let def = c.response(3)["result"].clone();
    assert_eq!(
        def["range"],
        json!({"start": {"line": 4, "character": 7}, "end": {"line": 4, "character": 10}})
    );
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
fn helper() -> i64 { 1 }
fn main() {
    print(helper());
    {
        print_bool(helper(2));
        /// The nested one.
        fn helper(x: i64) -> bool { x > 1 }
    }
    print(other());
}
fn other() -> i64 {
    fn helper() -> i64 { 3 }
    helper()
}
\nfn print<T: Int>(_x: T) {}\nfn print_bool(_b: bool) {}\n";
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
    let top = "```t\nfn helper: fn() -> i64\n```\n\n---\n\nThe top-level one.";
    let nested = "```t\nfn helper: fn(i64) -> bool\n```\n\n---\n\nThe nested one.";
    let others = "```t\nfn helper: fn() -> i64\n```";
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
#[derive(Discard)] struct P { x: i64 }
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
    #[derive(Discard)] struct P { z: i64 }
    top(local.z)
}
fn top(x: i64) -> P { P { x } }
\nfn print<T: Int>(_x: T) {}\nfn print_bool(_b: bool) {}\n";
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
struct Doc<'a> {
    uri: &'a str,
    src: &'a str,
    id: i64,
}

impl<'a> Doc<'a> {
    /// Opens `src` and checks it has the diagnostics `want` (by message).
    fn open(c: &mut Client, uri: &'a str, src: &'a str) -> (Self, Vec<Value>) {
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
fn helper() -> i64 { 1 }
fn main() with panic {
    print(helper());
    {
        print_bool(helper(2));
        // helper is nested here.
        fn helper(x: i64) -> bool { x > 1 }
    }
    let p = P { x: 1 };
    print(p.x + other());
    {
        let q = P { y: true };
        print_bool(q.y);
        #[derive(Discard)] struct P { y: bool }
    }
}
fn other() -> i64 {
    fn helper() -> i64 { 3 }
    helper()
}
#[derive(Discard)] struct P { x: i64 }
\nfn print<T: Int>(_x: T) {}\nfn print_bool(_b: bool) {}\n";
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
struct P { x: i64 }
fn main() {
    let x = 1;
    let p = P { x };
    let P { x } = p;
    print(x);
}
\nfn print<T: Int>(_x: T) {}\n";
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
        src.replace("{ x: i64", "{ y: i64")
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
#[derive(Discard)] enum Shape<T> {
    Circle(T),
    Empty,
}
#[derive(Discard)] struct Wrap<T> { inner: T }
fn area<T: Discard>(s: Shape<T>, w: Wrap<T>) -> T {
    match s {
        Shape::Circle(r) => r,
        Shape::Empty => w.inner,
    }
}
fn main() {
    print(area(Shape::Circle(2i64), Wrap { inner: 1 }));
    print(area(Shape::Empty, Wrap::<i64> { inner: 3 }));
}
\nfn print<T: Int>(_x: T) {}\n";
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

/// An effect parameter renames as a type parameter does, but not to a
/// built-in effect's name, which `with` would then mean.
#[test]
fn renames_effect_parameters() {
    let (child, mut c) = start();
    let src = "\
fn apply<effect e, F: Fn() -> i64 with e + Discard>(f: F) -> i64 with e { f() }
fn main() {
    print(apply(|| 1));
}
\nfn print<T: Int>(_x: T) {}\n";
    let (mut d, diags) = Doc::open(&mut c, "file:///tmp/effects.t", src);
    assert_eq!(diags, Vec::<Value>::new());
    assert_eq!(
        d.rename(&mut c, "e,", 0, "fx").unwrap(),
        src.replace("effect e,", "effect fx,")
            .replace("with e", "with fx")
    );
    let refused = d.rename(&mut c, "e,", 0, "panic").unwrap_err();
    assert_eq!(refused, "`panic` is a built-in effect");

    stop(child, c);
}

/// A field named in another field's refinement is renamed there too, and
/// a refinement's binder with its uses.
#[test]
fn renames_fields_in_refinements() {
    let (child, mut c) = start();
    let src = "\
#[derive(Copy, Discard)]
struct Span { lo: i64, hi: { h: i64 | h >= lo } }
fn main() {
    let s = Span { lo: 1, hi: 2 };
    print(s.hi - s.lo);
}
\nfn print<T: Int>(_x: T) {}\n";
    let (mut d, diags) = Doc::open(&mut c, "file:///tmp/fields.t", src);
    assert_eq!(diags, Vec::<Value>::new());
    assert_eq!(
        d.rename(&mut c, "lo", 0, "low").unwrap(),
        src.replace("lo", "low")
    );
    assert_eq!(
        d.rename(&mut c, "h", 1, "high").unwrap(),
        src.replace("h: i64 | h >=", "high: i64 | high >=")
    );

    stop(child, c);
}

/// What can't be renamed, and renames that are refused.
#[test]
fn refuses_bad_renames() {
    let (child, mut c) = start();
    let src = "\
#[derive(Discard)] struct P { x: i64 }
fn main() with panic {
    let a: i64 = 1;
    let b = 2;
    let p = P { x: 4 };
    fn helper() -> i64 { c() }
    print(a + b + p.x + helper());
}
fn c() -> i64 { 3 }
\nfn print<T: Int>(_x: T) {}\n";
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
        prepare(&mut c, "i64", 0),
        Err(json!("`i64` is a built-in type"))
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
    let a = prepare(&mut c, "a", 3);
    assert_eq!(a, Ok(d.range("a", 3)));

    assert_eq!(
        d.rename(&mut c, "a", 3, "1a"),
        Err("`1a` isn't a name".into())
    );
    assert_eq!(
        d.rename(&mut c, "a", 3, "a b"),
        Err("`a b` isn't a name".into())
    );
    assert_eq!(
        d.rename(&mut c, "a", 3, "match"),
        Err("`match` is a keyword".into())
    );
    assert_eq!(
        d.rename(&mut c, "a", 3, "_"),
        Err("`_` can't be a name".into())
    );
    assert_eq!(
        d.rename(&mut c, "P", 0, "i64"),
        Err("`i64` is a built-in type".into())
    );
    // A variable shadowed by another of the new name.
    assert_eq!(
        d.rename(&mut c, "a", 3, "b"),
        Err(
            "can't rename `a` to `b`: `b` on line 7 would refer to the variable `b` on line 4 \
             instead"
                .into()
        )
    );
    // A function that would be shadowed by the renamed variable.
    assert_eq!(
        d.rename(&mut c, "a", 3, "print"),
        Err(
            "can't rename `a` to `print`: line 7 would be an error: `i64` is not a function".into()
        )
    );
    // A nested `fn` would then capture it.
    assert_eq!(
        d.rename(&mut c, "a", 3, "c"),
        Err(
            "can't rename `a` to `c`: line 6 would be an error: can't capture `c` in a nested \
             `fn`; use a closure"
                .into()
        )
    );
    // A use of the renamed one would refer to another.
    assert_eq!(
        d.rename(&mut c, "c() -> i64 { 3", 0, "helper"),
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
        d.rename(&mut c, "c() -> i64 { 3", 0, "main"),
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
#[derive(Discard)] enum Option<T> {
    Some(T),
    None,
}
#[derive(Discard)] struct P { x: i64 }
fn get(o: Option<P>) -> i64 {
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
    print(id(2i64));
}
\nfn print<T: Int>(_x: T) {}\n";
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
        json!({"line": 0, "character": 24}),
        json!({"line": 4, "character": 26}),
    );
    // `o`s: 0 the parameter, 1 its use, 2 the variable, 3 its use.
    assert_eq!(ty(&mut c, "o:", 0), vec![option.clone(), p.clone()]);
    assert_eq!(ty(&mut c, "o))", 0), vec![option.clone(), p.clone()]);
    assert_eq!(ty(&mut c, "p)", 0), vec![p.clone()], "a pattern binding");
    assert_eq!(ty(&mut c, "get", 1), vec![option, p], "a function");
    assert_eq!(ty(&mut c, "x", 1), Vec::<Value>::new(), "an `i64` field");
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
    fn helper(x: i64) -> i64 {
        enum E { A(i64), B }
        fn inner() {}
        x
    }
    struct P { x: i64 }
}
enum Option<T> {
    Some(T),
    None,
}
\nfn print<T: Int>(_x: T) {}\n";
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
                        "fn(i64) -> i64",
                        [
                            [
                                "E",
                                10,
                                "enum E",
                                [["A", 22, "fn(i64) -> E", []], ["B", 22, "E", []],]
                            ],
                            ["inner", 12, "fn() -> ()", []],
                        ]
                    ],
                    ["P", 23, "struct P", [["x", 8, "i64", []]]],
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
            json!(["print", 12, "fn(T) -> ()", []]),
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
        "fn main() { let x: i64 = true; }\nstruct S {}\n",
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

#[test]
fn outlines_recover_around_syntax_errors_without_enabling_semantic_edits() {
    let (child, mut c) = start();
    let src = "\
fn before() {}
fn broken(x:) {}
struct Middle {}
fn bad_body() { let x = ; fn hidden() {} }
mod nested {
    fn kept() {}
    struct Broken { value: }
    enum End { Value }
}
pub impl Middle {}
fn main() {}
";
    let (mut d, diags) = Doc::open(&mut c, "file:///tmp/outline_syntax.t", src);
    let lines: Vec<_> = diags
        .iter()
        .map(|diag| {
            assert_eq!(diag["severity"], 1);
            assert!(
                diag["message"]
                    .as_str()
                    .unwrap()
                    .starts_with("syntax error:")
            );
            diag["range"]["start"]["line"].as_u64().unwrap()
        })
        .collect();
    assert_eq!(lines, [1, 3, 6, 9], "{diags:?}");

    fn names(symbols: &Value) -> Vec<Value> {
        symbols
            .as_array()
            .unwrap()
            .iter()
            .map(|s| json!([s["name"], names(&s["children"])]))
            .collect()
    }
    let response = d.ask(&mut c, "textDocument/documentSymbol", "main", 0, json!({}));
    let symbols = &response["result"];
    assert_eq!(
        names(symbols),
        vec![
            json!(["before", []]),
            json!(["Middle", []]),
            json!(["nested", [["kept", []], ["End", [["Value", []]]]]]),
            json!(["main", []]),
        ]
    );
    assert_eq!(symbols[1]["selectionRange"], d.range("Middle", 0));
    assert_eq!(symbols[3]["selectionRange"], d.range("main", 0));
    assert_eq!(
        symbols[2]["range"]["end"],
        json!({"line": 8, "character": 1})
    );
    // A later syntax error retains its own quick fix and source range.
    let response = d.ask(
        &mut c,
        "textDocument/codeAction",
        "pub",
        0,
        json!({"range": d.range("pub", 0), "context": {"diagnostics": []}}),
    );
    let actions = response["result"].as_array().unwrap();
    assert_eq!(actions.len(), 1, "{response}");
    assert_eq!(actions[0]["title"], "remove the `pub`");
    assert_eq!(actions[0]["diagnostics"][0], diags[3]);

    let response = d.ask(
        &mut c,
        "textDocument/formatting",
        "main",
        0,
        json!({"options": {"tabSize": 4, "insertSpaces": true}}),
    );
    assert!(response.get("error").is_none(), "{response}");
    assert_eq!(response["result"], Value::Null);
    let response = d.ask(&mut c, "textDocument/hover", "Middle", 0, json!({}));
    assert_eq!(response["result"], Value::Null);
    let response = d.ask(
        &mut c,
        "textDocument/rename",
        "Middle",
        0,
        json!({"newName": "Renamed"}),
    );
    assert!(response.get("error").is_some(), "{response}");

    // An edit must replace both the partial outline and all diagnostics.
    let fixed = "fn main() {}\n";
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
            "textDocument": {"uri": d.uri, "version": 2},
            "contentChanges": [{"text": fixed}]
        }}),
    );
    assert!(c.diagnostics().is_empty());
    let response = d.ask(&mut c, "textDocument/documentSymbol", "main", 0, json!({}));
    assert_eq!(names(&response["result"]), vec![json!(["main", []])]);
    assert_eq!(
        response["result"][0]["selectionRange"]["start"],
        json!({"line": 0, "character": 3})
    );
    stop(child, c);
}

#[test]
fn recovered_syntax_diagnostics_and_fixes_stay_in_the_module_file() {
    let (child, mut c) = start();
    let dir = std::env::temp_dir().join(format!("t-lsp-syntax-{}", std::process::id()));
    let main_uri = file_uri(&dir.join("main.t"));
    let module_uri = file_uri(&dir.join("helper.t"));
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": module_uri, "languageId": "t", "version": 1,
                "text": "fn before() {}\nfn broken(x:) {}\npub impl Before {}\nfn after() {}\n"}
        }}),
    );
    c.diagnostics();
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": main_uri, "languageId": "t", "version": 1,
                "text": "mod helper;\nfn main() {}\n"}
        }}),
    );
    c.send(
        json!({"jsonrpc": "2.0", "id": 2, "method": "textDocument/documentSymbol", "params": {
            "textDocument": {"uri": module_uri}
        }}),
    );
    let diags = c.diagnostics_before_response(2);
    assert_eq!(diags[&main_uri]["diagnostics"], json!([]));
    let module_diags = diags[&module_uri]["diagnostics"].as_array().unwrap();
    assert_eq!(module_diags.len(), 2, "{diags:?}");
    assert_eq!(
        module_diags[0]["range"]["start"],
        json!({"line": 1, "character": 12})
    );
    assert_eq!(
        module_diags[1]["range"]["start"],
        json!({"line": 2, "character": 0})
    );
    c.send(
        json!({"jsonrpc": "2.0", "id": 3, "method": "textDocument/codeAction", "params": {
            "textDocument": {"uri": module_uri},
            "range": {"start": {"line": 2, "character": 0}, "end": {"line": 2, "character": 3}},
            "context": {"diagnostics": []}
        }}),
    );
    let response = c.response(3);
    let edits = &response["result"][0]["edit"]["changes"];
    assert_eq!(edits.as_object().unwrap().len(), 1, "{response}");
    assert_eq!(
        edits[&module_uri][0],
        json!({
            "range": {"start": {"line": 2, "character": 0}, "end": {"line": 2, "character": 4}},
            "newText": ""
        })
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
    fn show(&self) -> i64;
}
#[derive(Discard)] struct P { x: i64 }
impl P {
    fn get(&self) -> i64 { self.x }
}
impl Show for P {
    fn show(&self) -> i64 { self.get() }
}
fn twice<T: Show + Discard>(t: T) -> i64 with panic { t.show() * 2 }
fn main() with panic {
    let p = P { x: 1 };
    print(twice(p));
    print(P { x: 2 }.show());
}
\nfn print<T: Int>(_x: T) {}\n";
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
        "```t\nfn get: fn(&P) -> i64\n```"
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
            json!(["print", 12, []]),
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
        fn area(&self) -> i64 with panic;
    }
    #[derive(Discard)]
    pub struct Sq { pub side: i64 }
    pub fn sq(side: i64) -> Sq { Sq { side } }
    pub mod more {
        pub fn twice(n: i64) -> i64 with panic { n * 2 }
    }
}
use shapes::{Area, Sq, more::twice as double};
impl Area for Sq {
    fn area(&self) -> i64 with panic { self.side * self.side }
}
fn main() with panic {
    let s: Sq = shapes::sq(3);
    print(s.area() + double(1) + shapes::more::twice(2));
}
\nfn print<T: Int>(_x: T) {}\n";
    let (mut d, diags) = Doc::open(&mut c, "file:///tmp/modules.t", src);
    assert_eq!(diags, Vec::<Value>::new());
    let hover = d.ask(&mut c, "textDocument/hover", "sq(3)", 0, json!({}));
    assert_eq!(
        hover["result"]["contents"]["value"],
        "```t\nfn sq: fn(i64) -> Sq\n```"
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
            json!(["print", 12, []]),
        ]
    );

    stop(child, c);
}

/// The next `publishDiagnostics` for `uri`, skipping others.
/// The `file:` URI of an absolute path, as an editor writes it:
/// `file:///tmp/a.t`, or on Windows `file:///C:/Users/a.t`.
fn file_uri(path: &std::path::Path) -> String {
    let path = path.to_string_lossy().replace('\\', "/");
    let path = match path.starts_with('/') {
        true => path,
        false => format!("/{path}"),
    };
    let mut out = String::from("file://");
    for (i, b) in path.bytes().enumerate() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b':' if i == 2 => out.push(':'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// A message the server can't make sense of, here a URI that isn't one,
/// doesn't stop it: a request gets an error, and the server goes on.
#[test]
fn survives_messages_it_cannot_read() {
    let (child, mut c) = start();
    let bad = r"file://C:\Users\me\main.t";
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": bad, "languageId": "t", "version": 1, "text": "fn main() {}\n"}
        }}),
    );
    c.send(
        json!({"jsonrpc": "2.0", "id": 2, "method": "textDocument/hover", "params": {
            "textDocument": {"uri": bad}, "position": {"line": 0, "character": 3}
        }}),
    );
    let r = c.response(2);
    assert!(r.get("error").is_some(), "{r}");
    // And it still answers about a file it can read.
    let uri = "file:///tmp/after.t";
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": uri, "languageId": "t", "version": 1, "text": "fn main() {}\n"}
        }}),
    );
    assert_eq!(diagnostics_for(&mut c, uri), Vec::<Value>::new());
    stop(child, c);
}

fn diagnostics_for(c: &mut Client, uri: &str) -> Vec<Value> {
    loop {
        let msg = c.recv();
        if msg["method"] == "textDocument/publishDiagnostics" && msg["params"]["uri"] == uri {
            return msg["params"]["diagnostics"].as_array().unwrap().clone();
        }
    }
}

/// A file of a module is checked as part of its program: the nearest
/// `main.t` above it that has it. Diagnostics go to the file they're in,
/// definitions and renames cross files, and an open file's text is used
/// over what's on disk.
#[test]
fn files_of_modules_are_checked_with_their_program() {
    let dir = std::env::temp_dir().join(format!("t-lsp-files-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let main =
        "mod util;\nfn main() {\n    print(util::helper());\n}\nfn print<T: Int>(_x: T) {}\n";
    std::fs::write(dir.join("main.t"), main).unwrap();
    std::fs::write(dir.join("util.t"), "pub fn helper() -> i64 { 1 }\n").unwrap();
    let main_uri = file_uri(&dir.join("main.t"));
    let util_uri = file_uri(&dir.join("util.t"));

    let (child, mut c) = start();
    // Opened with an error that isn't on disk: it's in `util.t`, and
    // `main.t`, not open, has none.
    let util = "pub fn helper() -> i64 {\n    true\n}\n";
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": util_uri, "languageId": "t", "version": 1, "text": util}
        }}),
    );
    let diags = diagnostics_for(&mut c, &util_uri);
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(
        diags[0]["range"]["start"],
        json!({"line": 1, "character": 4})
    );

    // Fixed, then `main.t` opened: going to `helper` goes to `util.t`.
    let util = "pub fn helper() -> i64 { 1 }\n";
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
            "textDocument": {"uri": util_uri, "version": 2},
            "contentChanges": [{"text": util}]
        }}),
    );
    assert_eq!(diagnostics_for(&mut c, &util_uri), Vec::<Value>::new());
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": main_uri, "languageId": "t", "version": 1, "text": main}
        }}),
    );
    assert_eq!(diagnostics_for(&mut c, &main_uri), Vec::<Value>::new());
    let at = |line, character| json!({"textDocument": {"uri": main_uri}, "position": {"line": line, "character": character}});
    c.send(json!({"jsonrpc": "2.0", "id": 2, "method": "textDocument/definition", "params": at(2, 17)}));
    let def = c.response(2);
    assert_eq!(def["result"]["uri"], util_uri, "{def}");
    assert_eq!(
        def["result"]["range"]["start"],
        json!({"line": 0, "character": 7})
    );
    // The name in `mod util;` goes to its file.
    c.send(
        json!({"jsonrpc": "2.0", "id": 3, "method": "textDocument/definition", "params": at(0, 5)}),
    );
    assert_eq!(c.response(3)["result"]["uri"], util_uri);

    // Renaming `helper` changes both files.
    let mut p = at(2, 17);
    p["newName"] = json!("assist");
    c.send(json!({"jsonrpc": "2.0", "id": 4, "method": "textDocument/rename", "params": p}));
    let r = c.response(4);
    let changes = &r["result"]["changes"];
    assert_eq!(
        changes[&main_uri],
        json!([{"range": {"start": {"line": 2, "character": 16}, "end": {"line": 2, "character": 22}}, "newText": "assist"}]),
        "{r}"
    );
    assert_eq!(
        changes[&util_uri],
        json!([{"range": {"start": {"line": 0, "character": 7}, "end": {"line": 0, "character": 13}}, "newText": "assist"}]),
        "{r}"
    );

    stop(child, c);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A program's packages are checked with it: going to a dependency's item
/// goes to its file, which is checked as a library of its own when it's
/// opened, with no `main`; what's wrong in a `t.toml` is said there.
#[test]
fn packages_are_checked_with_their_program() {
    let dir = std::env::temp_dir().join(format!("t-lsp-packages-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("app")).unwrap();
    std::fs::create_dir_all(dir.join("count")).unwrap();
    std::fs::create_dir_all(dir.join("broken")).unwrap();
    let main = "fn main() {\n    let _n = count::one();\n}\n";
    std::fs::write(dir.join("app/main.t"), main).unwrap();
    let toml = "[dependencies]\ncount = { path = \"../count\" }\n";
    std::fs::write(dir.join("app/t.toml"), toml).unwrap();
    let lib = "export fn one() -> i64 { 1 }\n";
    std::fs::write(dir.join("count/lib.t"), lib).unwrap();
    std::fs::write(dir.join("broken/main.t"), "fn main() {}\n").unwrap();
    let toml = "[dependencies]\ncount = { path = \"../count\", version = \"1\" }\n";
    std::fs::write(dir.join("broken/t.toml"), toml).unwrap();
    let main_uri = file_uri(&dir.join("app/main.t"));
    let lib_uri = file_uri(&dir.join("count/lib.t"));
    let broken_uri = file_uri(&dir.join("broken/main.t"));
    let toml_uri = file_uri(&dir.join("broken/t.toml"));

    let (child, mut c) = start();
    let open = |c: &mut Client, uri: &str, text: &str| {
        c.send(
            json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
                "textDocument": {"uri": uri, "languageId": "t", "version": 1, "text": text}
            }}),
        );
    };
    open(&mut c, &main_uri, main);
    assert_eq!(diagnostics_for(&mut c, &main_uri), Vec::<Value>::new());
    let at = json!({"textDocument": {"uri": main_uri}, "position": {"line": 1, "character": 20}});
    c.send(json!({"jsonrpc": "2.0", "id": 2, "method": "textDocument/definition", "params": at}));
    let def = c.response(2);
    assert_eq!(def["result"]["uri"], lib_uri, "{def}");
    open(&mut c, &lib_uri, lib);
    assert_eq!(diagnostics_for(&mut c, &lib_uri), Vec::<Value>::new());
    open(&mut c, &broken_uri, "fn main() {}\n");
    let diags = diagnostics_for(&mut c, &toml_uri);
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(
        diags[0]["range"]["start"],
        json!({"line": 1, "character": 29}),
        "{diags:?}"
    );

    stop(child, c);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn closing_unsaved_files_refreshes_the_programs_diagnostics() {
    let dir = std::env::temp_dir().join(format!("t-lsp-close-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let main = "mod util;\nmod consumer;\nfn main() { consumer::run(); }\n";
    std::fs::write(dir.join("main.t"), main).unwrap();
    std::fs::write(dir.join("util.t"), "pub fn helper() -> i64 { 1 }\n").unwrap();
    // This file stays closed, but errors induced by an open overlay
    // must be cleared here too when its on-disk dependency is restored.
    std::fs::write(
        dir.join("consumer.t"),
        "pub fn run() { let _x: i64 = crate::util::helper(); }\n",
    )
    .unwrap();
    let main_uri = file_uri(&dir.join("main.t"));
    let util_uri = file_uri(&dir.join("util.t"));
    let consumer_uri = file_uri(&dir.join("consumer.t"));
    let (child, mut c) = start();
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": main_uri, "languageId": "t", "version": 1, "text": main}
        }}),
    );
    assert!(diagnostics_for(&mut c, &main_uri).is_empty());

    for close_main in [false, true] {
        c.send(
            json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
                "textDocument": {"uri": util_uri, "languageId": "t", "version": 1,
                    "text": "pub fn helper() -> bool { true }\n"}
            }}),
        );
        let diags = diagnostics_for(&mut c, &consumer_uri);
        assert!(
            diags.iter().any(|d| d["message"]
                .as_str()
                .unwrap()
                .contains("expected `i64`, found `bool`")),
            "{diags:?}"
        );
        if close_main {
            c.send(
                json!({"jsonrpc": "2.0", "method": "textDocument/didClose", "params": {
                    "textDocument": {"uri": main_uri}
                }}),
            );
        }
        c.send(
            json!({"jsonrpc": "2.0", "method": "textDocument/didClose", "params": {
                "textDocument": {"uri": util_uri}
            }}),
        );
        c.send(
            json!({"jsonrpc": "2.0", "id": 2, "method": "textDocument/documentSymbol", "params": {
                "textDocument": {"uri": main_uri}
            }}),
        );
        let diags = c.diagnostics_before_response(2);
        assert_eq!(
            diags.get(&main_uri).map(|p| &p["diagnostics"]),
            Some(&json!([])),
            "{diags:?}"
        );
        assert_eq!(
            diags.get(&util_uri).map(|p| &p["diagnostics"]),
            Some(&json!([])),
            "{diags:?}"
        );
        assert_eq!(
            diags.get(&consumer_uri).map(|p| &p["diagnostics"]),
            Some(&json!([])),
            "{diags:?}"
        );
    }
    stop(child, c);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn closing_an_unsaved_root_can_detach_an_open_module() {
    let dir = std::env::temp_dir().join(format!("t-lsp-close-root-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("main.t"), "fn main() {}\n").unwrap();
    std::fs::write(dir.join("util.t"), "pub fn helper() -> bool { true }\n").unwrap();
    let main_uri = file_uri(&dir.join("main.t"));
    let util_uri = file_uri(&dir.join("util.t"));
    let (child, mut c) = start();
    for (uri, text) in [
        (
            &main_uri,
            "mod util;\nfn main() { let _x: i64 = util::helper(); }\n",
        ),
        (&util_uri, "pub fn helper() -> bool { true }\n"),
    ] {
        c.send(
            json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
                "textDocument": {"uri": uri, "languageId": "t", "version": 1, "text": text}
            }}),
        );
        assert!(!diagnostics_for(&mut c, &main_uri).is_empty());
    }
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didClose", "params": {
            "textDocument": {"uri": main_uri}
        }}),
    );
    c.send(
        json!({"jsonrpc": "2.0", "id": 2, "method": "textDocument/documentSymbol", "params": {
            "textDocument": {"uri": util_uri}
        }}),
    );
    let diags = c.diagnostics_before_response(2);
    assert_eq!(
        diags.get(&main_uri).map(|p| &p["diagnostics"]),
        Some(&json!([])),
        "{diags:?}"
    );
    let util = &diags[&util_uri]["diagnostics"];
    assert!(
        util.as_array().unwrap().iter().any(|d| d["message"]
            .as_str()
            .unwrap()
            .contains("program has no `main`")),
        "{util}"
    );
    stop(child, c);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A navigation target must be an ordinary readable file containing the
/// exact embedded bytes, with a UTF-16 range over the requested declaration.
/// Empty declaration/name means the start of a module's file.
fn library_location(
    location: &Value,
    library: &str,
    file: &str,
    declaration: &str,
    name: &str,
) -> std::path::PathBuf {
    let files = match library {
        "core" => t_lang::load::CORE_FILES,
        "alloc" => t_lang::load::ALLOC_FILES,
        "std" => t_lang::load::STD_FILES,
        _ => panic!("unknown library {library}"),
    };
    let embedded = files.iter().find(|(f, _)| *f == file).unwrap().1;
    let uri = location["uri"]
        .as_str()
        .unwrap_or_else(|| panic!("{location}"));
    assert!(uri.starts_with("file:///"), "{location}");
    let uri: lsp_types::Uri = uri.parse().unwrap();
    let decoded = uri.path().as_estr().decode().into_string().unwrap();
    let path = if cfg!(windows) {
        decoded.trim_start_matches('/')
    } else {
        &decoded
    };
    let path = std::path::PathBuf::from(path);
    assert!(
        path.ends_with(std::path::Path::new(library).join(file)),
        "{location}"
    );
    assert!(!path.starts_with(env!("CARGO_MANIFEST_DIR")), "{location}");
    assert_eq!(std::fs::read(&path).unwrap(), embedded.as_bytes());
    assert!(std::fs::metadata(&path).unwrap().permissions().readonly());
    let start = embedded.find(declaration).unwrap() + declaration.rfind(name).unwrap();
    let end = start + name.len();
    let position = |at: usize| {
        let before = &embedded[..at];
        json!({
            "line": before.matches('\n').count(),
            "character": before.rsplit('\n').next().unwrap().encode_utf16().count()
        })
    };
    assert_eq!(
        location["range"],
        json!({"start": position(start), "end": position(end)})
    );
    path
}

#[test]
fn navigates_library_functions_methods_types_and_imports() {
    let (child, mut c) = start();
    let src = "\
use core::mem::replace as exchange;
use core::mem;
use alloc::list::List;
use alloc::list;
use std::io::{Error, ErrorKind};
use std::io;
use std::Os;
#[derive(Discard)] struct Point { x: i64 }
fn main() with alloc {
    let mut value = 1;
    let _old = exchange(&mut value, 2);
    let mut items: List<Option<Point>> = List::new();
    items.push(Option::Some(Point { x: value }));
    let _ = items.pop();
    let error = Error::new(ErrorKind::Other);
    let _ = error.kind();
}
";
    let (mut d, diagnostics) = Doc::open(&mut c, "file:///tmp/library-navigation.t", src);
    assert!(
        diagnostics.iter().all(|d| d["severity"] != 1),
        "{diagnostics:?}"
    );
    for (needle, nth, library, file, declaration, name) in [
        ("replace as", 0, "core", "mem.t", "fn replace", "replace"),
        ("exchange(", 0, "core", "mem.t", "fn replace", "replace"),
        ("mem;", 0, "core", "mem.t", "", ""),
        ("Option<", 0, "core", "option.t", "enum Option", "Option"),
        (
            "Some(",
            0,
            "core",
            "option.t",
            "enum Option<T> { None, Some",
            "Some",
        ),
        ("List;", 0, "alloc", "list.t", "struct List", "List"),
        ("List<", 0, "alloc", "list.t", "struct List", "List"),
        ("list;", 0, "alloc", "list.t", "", ""),
        ("push(", 0, "alloc", "list.t", "fn push", "push"),
        ("Error,", 0, "std", "io.t", "struct Error", "Error"),
        (
            "ErrorKind::Other",
            0,
            "std",
            "io.t",
            "enum ErrorKind",
            "ErrorKind",
        ),
        ("io;", 0, "std", "io.t", "", ""),
        ("kind()", 0, "std", "io.t", "fn kind", "kind"),
        ("Os;", 0, "std", "lib.t", "struct Os", "Os"),
        ("core::mem::", 0, "core", "lib.t", "", ""),
        ("alloc::list::", 0, "alloc", "lib.t", "", ""),
        ("std::io::", 0, "std", "lib.t", "", ""),
    ] {
        let response = d.ask(&mut c, "textDocument/definition", needle, nth, json!({}));
        library_location(&response["result"], library, file, declaration, name);
    }

    // Type definition includes library and program types together, in order.
    let response = d.ask(
        &mut c,
        "textDocument/typeDefinition",
        "items:",
        0,
        json!({}),
    );
    let defs = response["result"].as_array().unwrap();
    assert_eq!(defs.len(), 3, "{response}");
    library_location(&defs[0], "alloc", "list.t", "struct List", "List");
    library_location(&defs[1], "core", "option.t", "enum Option", "Option");
    assert_eq!(defs[2]["uri"], d.uri);
    assert_eq!(defs[2]["range"], d.range("Point", 0));
    let response = d.ask(
        &mut c,
        "textDocument/typeDefinition",
        "error =",
        0,
        json!({}),
    );
    let defs = response["result"].as_array().unwrap();
    assert_eq!(defs.len(), 1, "{response}");
    library_location(&defs[0], "std", "io.t", "struct Error", "Error");

    // Navigation does not make library declarations renameable from a program.
    for needle in ["exchange(", "List;", "kind()"] {
        for method in ["textDocument/prepareRename", "textDocument/rename"] {
            let response = d.ask(&mut c, method, needle, 0, json!({"newName": "changed"}));
            assert!(
                response["error"]["message"]
                    .as_str()
                    .unwrap()
                    .contains("can't be changed"),
                "{response}"
            );
        }
    }
    // Program references continue to exclude library declarations.
    assert_eq!(d.references(&mut c, "exchange(", 0, true).len(), 2);
    stop(child, c);
}

#[test]
fn browses_open_library_sources_without_editing_them() {
    let (child, mut c) = start();
    let (mut program, diagnostics) = Doc::open(
        &mut c,
        "file:///tmp/browse-library.t",
        "use alloc::list::List;\nfn main() { let _ = List::<i64>::new(); }\n",
    );
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let target = program.ask(&mut c, "textDocument/definition", "List;", 0, json!({}));
    let path = library_location(&target["result"], "alloc", "list.t", "struct List", "List");
    let uri = target["result"]["uri"].as_str().unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    let (mut d, diagnostics) = Doc::open(&mut c, uri, &text);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let r = d.ask(&mut c, "textDocument/definition", "RawBuf;", 0, json!({}));
    library_location(&r["result"], "alloc", "raw.t", "struct RawBuf", "RawBuf");
    let r = d.ask(&mut c, "textDocument/definition", "empty()", 0, json!({}));
    library_location(&r["result"], "alloc", "raw.t", "fn empty", "empty");
    let r = d.ask(&mut c, "textDocument/definition", "len }", 0, json!({}));
    assert_eq!(r["result"]["uri"], uri, "{r}");
    let start = d.at("len: { n", 0);
    assert_eq!(
        r["result"]["range"],
        json!({
            "start": start,
            "end": {"line": start["line"], "character": start["character"].as_u64().unwrap() + 3}
        })
    );
    let r = d.ask(&mut c, "textDocument/hover", "RawBuf;", 0, json!({}));
    assert!(
        r["result"]["contents"]["value"]
            .as_str()
            .unwrap()
            .contains("RawBuf"),
        "{r}"
    );
    let r = d.ask(&mut c, "textDocument/typeDefinition", "buf:", 0, json!({}));
    library_location(&r["result"][0], "alloc", "raw.t", "struct RawBuf", "RawBuf");
    for method in ["textDocument/prepareRename", "textDocument/rename"] {
        let r = d.ask(&mut c, method, "len: { n", 0, json!({"newName": "length"}));
        assert!(
            r["error"]["message"]
                .as_str()
                .unwrap()
                .contains("can't be changed"),
            "{r}"
        );
    }
    let r = d.ask(
        &mut c,
        "textDocument/formatting",
        "List",
        0,
        json!({"options": {"tabSize": 4, "insertSpaces": true}}),
    );
    assert_eq!(r["result"], Value::Null);
    let r = d.ask(
        &mut c,
        "textDocument/codeAction",
        "List",
        0,
        json!({
            "range": {"start": {"line": 0, "character": 0}, "end": {"line": 1000, "character": 0}},
            "context": {"diagnostics": []}
        }),
    );
    assert_eq!(r["result"], json!([]));
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
            "textDocument": {"uri": uri, "version": 2}, "contentChanges": [{"text": "fn broken("}]
        }}),
    );
    assert!(c.diagnostics().is_empty());
    let r = d.ask(&mut c, "textDocument/definition", "RawBuf;", 0, json!({}));
    library_location(&r["result"], "alloc", "raw.t", "struct RawBuf", "RawBuf");
    assert_eq!(std::fs::read_to_string(path).unwrap(), text);
    stop(child, c);
}

#[test]
fn installed_binary_navigates_its_embedded_sources_outside_the_checkout() {
    let dir = std::env::temp_dir().join(format!("t-lsp-installed-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("temporary sources")).unwrap();
    // A nearby library checkout must never override the embedded version.
    std::fs::create_dir_all(dir.join("core")).unwrap();
    std::fs::write(dir.join("core/option.t"), "wrong library version\r\n").unwrap();
    let binary = dir.join(format!("installed t{}", std::env::consts::EXE_SUFFIX));
    std::fs::copy(env!("CARGO_BIN_EXE_t"), &binary).unwrap();
    let mut command = Command::new(binary);
    command.current_dir(&dir);
    for key in ["TMPDIR", "TMP", "TEMP"] {
        command.env(key, dir.join("temporary sources"));
    }
    let (child, mut c) = start_command(command);
    let (mut d, diagnostics) = Doc::open(
        &mut c,
        "file:///tmp/installed-navigation.t",
        "fn main() { let _o: Option<i64> = Option::Some(1); }\n",
    );
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let r = d.ask(&mut c, "textDocument/definition", "Option", 0, json!({}));
    let path = library_location(&r["result"], "core", "option.t", "enum Option", "Option");
    assert!(path.starts_with(dir.join("temporary sources")), "{path:?}");
    assert!(r["result"]["uri"].as_str().unwrap().contains("%20"));
    stop(child, c);
    assert!(
        std::fs::read_to_string(&path)
            .unwrap()
            .contains("enum Option")
    );
    // Windows requires clearing the read-only bit before removing test files.
    #[cfg(windows)]
    fn writable_tree(dir: &std::path::Path) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                writable_tree(&path);
            } else {
                let mut permissions = std::fs::metadata(&path).unwrap().permissions();
                permissions.set_readonly(false);
                std::fs::set_permissions(path, permissions).unwrap();
            }
        }
    }
    #[cfg(windows)]
    writable_tree(&dir);
    std::fs::remove_dir_all(dir).unwrap();
}

/// The core library's items have hovers with their doc comments and
/// definitions in the embedded source. A `for` loop
/// hovers like the variables it binds, and nothing of what it means.
#[test]
fn hovers_and_navigates_the_core_library() {
    let (child, mut c) = start();
    let uri = "file:///tmp/core.t";
    let src = "fn main() {\n    let o = Option::Some(1);\n    for i in 0..3i64 {\n        print(o.unwrap_or(i));\n    }\n}\nfn print<T: Int>(_x: T) {}\n";
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": uri, "languageId": "t", "version": 1, "text": src}
        }}),
    );
    assert_eq!(c.diagnostics(), Vec::<Value>::new());
    let at = |line: u32, character: u32| json!({"textDocument": {"uri": uri}, "position": {"line": line, "character": character}});

    c.send(json!({"jsonrpc": "2.0", "id": 2, "method": "textDocument/hover", "params": at(3, 17)}));
    let hover = c.response(2)["result"]["contents"]["value"].clone();
    let hover = hover.as_str().unwrap();
    assert!(hover.contains("unwrap_or"), "{hover}");
    assert!(
        hover.contains("What's in it, or else `default`."),
        "{hover}"
    );

    c.send(json!({"jsonrpc": "2.0", "id": 3, "method": "textDocument/definition", "params": at(3, 17)}));
    library_location(
        &c.response(3)["result"],
        "core",
        "option.t",
        "fn unwrap_or(",
        "unwrap_or",
    );

    // The loop's variable, and nothing on `for` or `in`.
    c.send(json!({"jsonrpc": "2.0", "id": 4, "method": "textDocument/hover", "params": at(2, 8)}));
    let hover = c.response(4)["result"]["contents"]["value"].clone();
    assert_eq!(hover, "```t\ni: i64\n```");
    for (id, character) in [(5, 5), (6, 11)] {
        let params = at(2, character);
        c.send(
            json!({"jsonrpc": "2.0", "id": id, "method": "textDocument/hover", "params": params}),
        );
        assert_eq!(c.response(id)["result"], Value::Null, "at {character}");
    }

    stop(child, c);
}

/// The alloc library's items hover and navigate as the core library's do;
/// `unsafe` blocks hover as nothing, and their warnings
/// are diagnostics.
#[test]
fn hovers_and_navigates_the_alloc_library() {
    let (child, mut c) = start();
    let uri = "file:///tmp/alloc.t";
    let src = "use alloc::raw::RawBuf;\n\nfn main() with alloc {\n    let b: RawBuf<i64> = RawBuf::new(1);\n    unsafe { b.free() };\n}\n";
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": uri, "languageId": "t", "version": 1, "text": src}
        }}),
    );
    let diagnostics = c.diagnostics();
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    let message = diagnostics[0]["message"].as_str().unwrap();
    assert!(message.contains("no `// SAFETY:` comment"), "{message}");
    let at = |line: u32, character: u32| json!({"textDocument": {"uri": uri}, "position": {"line": line, "character": character}});

    c.send(json!({"jsonrpc": "2.0", "id": 2, "method": "textDocument/hover", "params": at(4, 16)}));
    let hover = c.response(2)["result"]["contents"]["value"].clone();
    let hover = hover.as_str().unwrap();
    assert!(hover.contains("free"), "{hover}");
    assert!(hover.contains("Gives its memory back."), "{hover}");

    c.send(json!({"jsonrpc": "2.0", "id": 3, "method": "textDocument/definition", "params": at(4, 16)}));
    library_location(
        &c.response(3)["result"],
        "alloc",
        "raw.t",
        "fn free",
        "free",
    );

    c.send(json!({"jsonrpc": "2.0", "id": 4, "method": "textDocument/hover", "params": at(4, 6)}));
    assert_eq!(c.response(4)["result"], Value::Null);

    stop(child, c);
}

/// A list's methods hover with their docs, from `alloc/list.t`.
#[test]
fn hovers_list_methods() {
    let (child, mut c) = start();
    let uri = "file:///tmp/list.t";
    let src = "use alloc::list::List;\n\nfn main() with {alloc, panic} {\n    let mut l: List<i64> = List::new();\n    l.push(1);\n    print(l.iter().count());\n}\nfn print<T: Int>(_x: T) {}\n";
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": uri, "languageId": "t", "version": 1, "text": src}
        }}),
    );
    let diagnostics = c.diagnostics();
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let at = |line: u32, character: u32| json!({"textDocument": {"uri": uri}, "position": {"line": line, "character": character}});

    c.send(json!({"jsonrpc": "2.0", "id": 2, "method": "textDocument/hover", "params": at(4, 7)}));
    let hover = c.response(2)["result"]["contents"]["value"].clone();
    let hover = hover.as_str().unwrap();
    assert!(hover.contains("push"), "{hover}");
    assert!(hover.contains("Puts `v` at the end."), "{hover}");

    c.send(json!({"jsonrpc": "2.0", "id": 3, "method": "textDocument/hover", "params": at(5, 14)}));
    let hover = c.response(3)["result"]["contents"]["value"].clone();
    let hover = hover.as_str().unwrap();
    assert!(hover.contains("Iter<"), "{hover}");
    assert!(hover.contains("front to back"), "{hover}");

    stop(child, c);
}

/// A slice's methods, in `impl<T> [T]`, hover with their docs, from
/// `core/slice.t`, and a slice's type as it's written.
#[test]
fn hovers_slice_methods() {
    let (child, mut c) = start();
    let uri = "file:///tmp/slice.t";
    let src = "fn main() {\n    let s: &[i64] = core::slice::empty();\n    print(s.len());\n}\nfn print<T: Int>(_x: T) {}\n";
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": uri, "languageId": "t", "version": 1, "text": src}
        }}),
    );
    let diagnostics = c.diagnostics();
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let at = |line: u32, character: u32| json!({"textDocument": {"uri": uri}, "position": {"line": line, "character": character}});

    c.send(json!({"jsonrpc": "2.0", "id": 2, "method": "textDocument/hover", "params": at(2, 13)}));
    let hover = c.response(2)["result"]["contents"]["value"].clone();
    let hover = hover.as_str().unwrap();
    assert!(hover.contains("len"), "{hover}");
    assert!(hover.contains("How many values it has."), "{hover}");

    c.send(json!({"jsonrpc": "2.0", "id": 3, "method": "textDocument/hover", "params": at(2, 10)}));
    let hover = c.response(3)["result"]["contents"]["value"].clone();
    let hover = hover.as_str().unwrap();
    assert!(hover.contains("&[i64]"), "{hover}");

    stop(child, c);
}

#[test]
fn hovers_strings_and_characters() {
    let (child, mut c) = start();
    let uri = "file:///tmp/str.t";
    let src = "fn main() {\n    let s = \"a\\u{20AC}\";\n    print(s.len());\n    let c = 'x';\n    print(c as u32);\n}\nfn print<T: Int>(_x: T) {}\n";
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": uri, "languageId": "t", "version": 1, "text": src}
        }}),
    );
    let diagnostics = c.diagnostics();
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let at = |line: u32, character: u32| json!({"textDocument": {"uri": uri}, "position": {"line": line, "character": character}});

    c.send(json!({"jsonrpc": "2.0", "id": 2, "method": "textDocument/hover", "params": at(1, 8)}));
    let hover = c.response(2)["result"]["contents"]["value"].clone();
    let hover = hover.as_str().unwrap();
    assert!(hover.contains("&'static str"), "{hover}");

    c.send(json!({"jsonrpc": "2.0", "id": 3, "method": "textDocument/hover", "params": at(2, 13)}));
    let hover = c.response(3)["result"]["contents"]["value"].clone();
    let hover = hover.as_str().unwrap();
    assert!(hover.contains("How many bytes it has"), "{hover}");

    c.send(json!({"jsonrpc": "2.0", "id": 4, "method": "textDocument/hover", "params": at(3, 8)}));
    let hover = c.response(4)["result"]["contents"]["value"].clone();
    let hover = hover.as_str().unwrap();
    assert!(hover.contains("char"), "{hover}");

    stop(child, c);
}

#[test]
fn disk_dependencies_refresh_on_requests_and_file_notifications() {
    let dir = std::env::temp_dir().join(format!("t-lsp-disk-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let main = "mod util;\nmod consumer;\nfn main() { consumer::run(); }\n";
    std::fs::write(dir.join("main.t"), main).unwrap();
    let helper = "pub fn helper() -> i64 { 1 }\n";
    std::fs::write(dir.join("util.t"), helper).unwrap();
    std::fs::write(
        dir.join("consumer.t"),
        "pub fn run() { let _x: i64 = crate::util::helper(); }\n",
    )
    .unwrap();
    let main_uri = file_uri(&dir.join("main.t"));
    let util_uri = file_uri(&dir.join("util.t"));
    let consumer_uri = file_uri(&dir.join("consumer.t"));
    let (child, mut c) = start();
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": main_uri, "languageId": "t", "version": 1, "text": main}
        }}),
    );
    assert!(diagnostics_for(&mut c, &main_uri).is_empty());
    let request = |id| {
        json!({"jsonrpc": "2.0", "id": id, "method": "textDocument/hover", "params": {
            "textDocument": {"uri": main_uri}, "position": {"line": 2, "character": 24}
        }})
    };
    // An unrelated disk edit neither rechecks nor republishes this program.
    std::fs::write(dir.join("unrelated.t"), "fn main() {}\n").unwrap();
    c.send(request(2));
    assert!(c.diagnostics_before_response(2).is_empty());

    // A closed dependency's edit is observed without a client notification.
    // Equal-length text also rules out using file size as a freshness check.
    let changed = "pub fn helper()-> bool{true}\n";
    assert_eq!(changed.len(), helper.len());
    let modified = std::fs::metadata(dir.join("util.t"))
        .unwrap()
        .modified()
        .unwrap();
    std::fs::write(dir.join("util.t"), changed).unwrap();
    std::fs::File::options()
        .write(true)
        .open(dir.join("util.t"))
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(modified))
        .unwrap();
    c.send(request(3));
    let diags = c.diagnostics_before_response(3);
    assert!(
        diags[&consumer_uri]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| {
                d["message"]
                    .as_str()
                    .unwrap()
                    .contains("expected `i64`, found `bool`")
            }),
        "{diags:?}"
    );

    // A notification can refresh diagnostics without a semantic request.
    std::fs::write(dir.join("util.t"), helper).unwrap();
    c.send(
        json!({"jsonrpc": "2.0", "method": "workspace/didChangeWatchedFiles", "params": {
            "changes": [{"uri": util_uri, "type": 2}]
        }}),
    );
    assert!(diagnostics_for(&mut c, &consumer_uri).is_empty());
    c.send(request(4));
    c.response(4);

    // Removing a module reports its load error and clears diagnostics for
    // files that loading can no longer reach. Restoring it invalidates the
    // failed snapshot, including the read of the missing file.
    std::fs::remove_file(dir.join("util.t")).unwrap();
    c.send(request(5));
    let diags = c.diagnostics_before_response(5);
    assert!(
        diags[&main_uri]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| { d["message"].as_str().unwrap().contains("can't read") }),
        "{diags:?}"
    );
    assert_eq!(diags[&consumer_uri]["diagnostics"], json!([]));
    std::fs::write(dir.join("util.t"), helper).unwrap();
    c.send(request(6));
    let diags = c.diagnostics_before_response(6);
    assert_eq!(diags[&main_uri]["diagnostics"], json!([]));
    stop(child, c);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn editing_a_root_detaches_and_reattaches_open_modules() {
    let dir = std::env::temp_dir().join(format!("t-lsp-membership-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let main = "mod util;\nfn main() { let _n: i64 = util::helper(); }\n";
    let util = "pub fn helper() -> bool { true }\n";
    std::fs::write(dir.join("main.t"), main).unwrap();
    std::fs::write(dir.join("util.t"), util).unwrap();
    let main_uri = file_uri(&dir.join("main.t"));
    let util_uri = file_uri(&dir.join("util.t"));
    let (child, mut c) = start();
    for (uri, text) in [(&main_uri, main), (&util_uri, util)] {
        c.send(
            json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
                "textDocument": {"uri": uri, "languageId": "t", "version": 1, "text": text}
            }}),
        );
        assert!(!diagnostics_for(&mut c, &main_uri).is_empty());
    }
    for (version, text, attached) in [(2, "fn main() {}\n", false), (3, main, true)] {
        c.send(json!({"jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
            "textDocument": {"uri": main_uri, "version": version}, "contentChanges": [{"text": text}]
        }}));
        c.send(
            json!({"jsonrpc": "2.0", "id": version, "method": "textDocument/hover", "params": {
                "textDocument": {"uri": util_uri}, "position": {"line": 0, "character": 8}
            }}),
        );
        let diags = c.diagnostics_before_response(version);
        let util_diags = diags[&util_uri]["diagnostics"].as_array().unwrap();
        if attached {
            assert!(util_diags.is_empty(), "{diags:?}");
            assert!(
                !diags[&main_uri]["diagnostics"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
        } else {
            assert_eq!(diags[&main_uri]["diagnostics"], json!([]));
            assert!(
                util_diags.iter().any(|d| d["message"]
                    .as_str()
                    .unwrap()
                    .contains("program has no `main`")),
                "{diags:?}"
            );
        }
    }
    stop(child, c);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_new_nearer_root_changes_an_open_modules_program() {
    let dir = std::env::temp_dir().join(format!("t-lsp-nearer-root-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("nested")).unwrap();
    let outer =
        "mod nested { pub mod util; }\nfn main() { let _n: i64 = nested::util::helper(); }\n";
    let util = "pub fn helper() -> i64 { 1 }\n";
    std::fs::write(dir.join("main.t"), outer).unwrap();
    std::fs::write(dir.join("nested/util.t"), util).unwrap();
    let outer_uri = file_uri(&dir.join("main.t"));
    let nearer_uri = file_uri(&dir.join("nested/main.t"));
    let util_uri = file_uri(&dir.join("nested/util.t"));
    let (child, mut c) = start();
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": util_uri, "languageId": "t", "version": 1, "text": util}
        }}),
    );
    assert!(diagnostics_for(&mut c, &util_uri).is_empty());
    std::fs::write(
        dir.join("nested/main.t"),
        "mod util;\nfn main() { let _b: bool = util::helper(); }\n",
    )
    .unwrap();
    c.send(
        json!({"jsonrpc": "2.0", "id": 2, "method": "textDocument/hover", "params": {
            "textDocument": {"uri": util_uri}, "position": {"line": 0, "character": 8}
        }}),
    );
    let diags = c.diagnostics_before_response(2);
    assert!(
        diags[&nearer_uri]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| {
                d["message"]
                    .as_str()
                    .unwrap()
                    .contains("expected `bool`, found `i64`")
            }),
        "{diags:?}"
    );
    assert_eq!(diags[&outer_uri]["diagnostics"], json!([]));
    stop(child, c);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn check_run_and_lsp_agree_on_semantic_acceptance() {
    let dir = std::env::temp_dir().join(format!("t-lsp-acceptance-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("main.t");
    let uri = file_uri(&path);
    let cases = [
        (
            "fn id<T: Copy>(x: T) -> T { x }\nfn main() { let _n = id(42i64); }",
            true,
        ),
        (include_str!("t/err_arg_type.t"), false),
        (include_str!("t/err_effect_unwrap.t"), false),
        (include_str!("t/err_refine_arg.t"), false),
        (include_str!("t/err_borrow_assign.t"), false),
    ];
    let (child, mut c) = start();
    for (index, (src, accepted)) in cases.into_iter().enumerate() {
        std::fs::write(&path, src).unwrap();
        for command in ["check", "run"] {
            let output = Command::new(env!("CARGO_BIN_EXE_t"))
                .arg(command)
                .arg(&path)
                .output()
                .unwrap();
            assert_eq!(
                output.status.success(),
                accepted,
                "{command}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        c.send(json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": uri, "languageId": "t", "version": index as i32, "text": src}
        }}));
        let diags = diagnostics_for(&mut c, &uri);
        assert_eq!(
            !diags.iter().any(|d| d["severity"] == 1),
            accepted,
            "{diags:?}"
        );
    }
    stop(child, c);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn refreshing_one_root_preserves_another_roots_diagnostics_in_shared_files() {
    let dir = std::env::temp_dir().join(format!("t-lsp-shared-file-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("nested")).unwrap();
    let outer = "mod nested { pub mod util; }\npub fn value() -> i64 { 1 }\nfn main() { let _ = nested::util::helper(); }\n";
    let inner =
        "mod util;\npub fn value() -> bool { true }\nfn main() { let _ = util::helper(); }\n";
    let util = "pub fn helper() -> i64 { crate::value() }\n";
    std::fs::write(dir.join("main.t"), outer).unwrap();
    std::fs::write(dir.join("nested/main.t"), inner).unwrap();
    std::fs::write(dir.join("nested/util.t"), util).unwrap();
    let outer_uri = file_uri(&dir.join("main.t"));
    let inner_uri = file_uri(&dir.join("nested/main.t"));
    let util_uri = file_uri(&dir.join("nested/util.t"));
    let (child, mut c) = start();
    for (uri, text) in [(&outer_uri, outer), (&inner_uri, inner)] {
        c.send(
            json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
                "textDocument": {"uri": uri, "languageId": "t", "version": 1, "text": text}
            }}),
        );
    }
    assert!(!diagnostics_for(&mut c, &util_uri).is_empty());
    // Only the outer root changes. Its successful analysis must not clear
    // the error the unchanged inner root still finds in the shared file.
    c.send(json!({"jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
        "textDocument": {"uri": outer_uri, "version": 2}, "contentChanges": [{"text": format!("{outer}\n")}]
    }}));
    c.send(
        json!({"jsonrpc": "2.0", "id": 2, "method": "textDocument/hover", "params": {
            "textDocument": {"uri": inner_uri}, "position": {"line": 0, "character": 5}
        }}),
    );
    let diags = c.diagnostics_before_response(2);
    assert!(
        diags[&util_uri]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| {
                d["message"]
                    .as_str()
                    .unwrap()
                    .contains("expected `i64`, found `bool`")
            }),
        "{diags:?}"
    );
    stop(child, c);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn hovers_and_finds_type_aliases() {
    let (child, mut c) = start();
    let uri = "file:///tmp/alias.t";
    let src = "\
/// A distance.
type Meters = i64;
type Pair<T> = Option<T>;
fn walk(m: Meters) -> Pair<Meters> { Option::Some(m) }
fn main() {
    let _p: Pair<bool> = Option::None;
    let _w = walk(3);
}
";
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": uri, "languageId": "t", "version": 1, "text": src}
        }}),
    );
    assert_eq!(c.diagnostics(), Vec::<Value>::new());
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

    // Where it's declared, with its doc comment, and where it's used: what
    // it stands for there.
    let hover = ask(&mut c, "textDocument/hover", "Meters", 0);
    assert_eq!(
        hover["contents"]["value"], "```t\ntype Meters = i64\n```\n\n---\n\nA distance.",
        "{hover}"
    );
    let hover = ask(&mut c, "textDocument/hover", "Pair", 2);
    assert_eq!(
        hover["contents"]["value"], "```t\ntype Pair = Option<bool>\n```",
        "{hover}"
    );
    let def = ask(&mut c, "textDocument/definition", "Meters", 1);
    assert_eq!(def["range"]["start"], at("Meters", 0), "{def}");

    stop(child, c);
}

/// Renaming an alias renames its uses, through a path and a `use` too, and
/// nothing named the same that it stands for.
#[test]
fn renames_type_aliases() {
    let (child, mut c) = start();
    let src = "\
mod m {
    pub type Num = i64;
}
use m::Num;
fn twice(n: Num) -> m::Num { n * 2 }
fn main() {
    print(twice(2));
}
\nfn print<T: Int>(_x: T) {}\n";
    let (mut d, diags) = Doc::open(&mut c, "file:///tmp/alias.t", src);
    assert_eq!(diags, Vec::<Value>::new());
    assert_eq!(
        d.rename(&mut c, "Num", 0, "Count").unwrap(),
        src.replace("Num", "Count")
    );
    assert_eq!(
        d.references(&mut c, "Num", 2, true).len(),
        4,
        "the declaration, the `use`, and both uses"
    );

    stop(child, c);
}

/// A package from git is fetched for the editor as for `t check`, and
/// going to one of its items goes to its checkout.
#[test]
fn packages_from_git_are_fetched_and_navigated() {
    let dir = std::env::temp_dir().join(format!("t-lsp-git-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let repo = dir.join("count");
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::write(repo.join("lib.t"), "export fn one() -> i64 { 1 }\n").unwrap();
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .current_dir(&repo)
            .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
            .args(["-c", "commit.gpgsign=false"])
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    };
    git(&["init", "-q"]);
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "one"]);
    let rev = git(&["rev-parse", "HEAD"]);
    std::fs::create_dir_all(dir.join("app")).unwrap();
    let main = "fn main() {\n    let _n = count::one();\n}\n";
    std::fs::write(dir.join("app/main.t"), main).unwrap();
    let path = repo.display().to_string().replace('\\', "/");
    let url = match path.starts_with('/') {
        true => format!("file://{path}"),
        false => format!("file:///{path}"),
    };
    let toml = format!("[dependencies]\ncount = {{ git = \"{url}\", rev = \"{rev}\" }}\n");
    std::fs::write(dir.join("app/t.toml"), toml).unwrap();
    let main_uri = file_uri(&dir.join("app/main.t"));

    let mut command = Command::new(env!("CARGO_BIN_EXE_t"));
    let cache = dir.join("cache");
    command.env("T_LANG_CACHE", &cache);
    let (child, mut c) = start_command(command);
    c.send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": main_uri, "languageId": "t", "version": 1, "text": main}
        }}),
    );
    assert_eq!(diagnostics_for(&mut c, &main_uri), Vec::<Value>::new());
    let at = json!({"textDocument": {"uri": main_uri}, "position": {"line": 1, "character": 20}});
    c.send(json!({"jsonrpc": "2.0", "id": 2, "method": "textDocument/definition", "params": at}));
    let def = c.response(2);
    let uri = def["result"]["uri"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    assert!(uri.starts_with(&file_uri(&cache.join("git"))), "{def}");
    assert!(uri.ends_with(&format!("/{rev}/lib.t")), "{def}");

    stop(child, c);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// What `if let`, `while let` and `let .. else` bind hovers, goes to its
/// definition and renames like any variable; and the `let`, where the
/// `else` they mean is made up, has nothing to show or rename.
#[test]
fn hovers_and_renames_through_if_let_and_let_else() {
    let (child, mut c) = start();
    let src = "\
enum Shape { Circle(i64), Rect(i64, i64) }
fn area(s: Option<Shape>) -> i64 {
    let Option::Some(shape) = s else { return 0 };
    if let Shape::Circle(r) = shape { r * r } else { 1 }
}
fn count(mut it: core::ops::Range<i64>) -> i64 {
    let mut n = 0;
    while let Option::Some(k) = it.next() {
        n = n + k;
    }
    n
}
fn main() {
    let _a = area(Option::Some(Shape::Circle(2)));
    let _b = count(0..3);
    let _c = area(Option::Some(Shape::Rect(1, 2)));
}
";
    let (mut d, diags) = Doc::open(&mut c, "file:///tmp/iflet.t", src);
    assert_eq!(diags, Vec::<Value>::new());
    let hover = d.ask(&mut c, "textDocument/hover", "shape", 1, json!({}));
    assert_eq!(
        hover["result"]["contents"]["value"], "```t\nshape: Shape\n```",
        "{hover}"
    );
    let hover = d.ask(&mut c, "textDocument/hover", "r * r", 0, json!({}));
    assert_eq!(
        hover["result"]["contents"]["value"], "```t\nr: i64\n```",
        "{hover}"
    );
    let def = d.ask(&mut c, "textDocument/definition", "k;", 0, json!({}));
    assert_eq!(def["result"]["range"], d.range("k", 0), "{def}");
    // The made-up `else` arms name `Option` and `Shape` where the `let` is,
    // as no text of their own.
    for nth in 0..3 {
        let hover = d.ask(&mut c, "textDocument/hover", "let", nth, json!({}));
        assert_eq!(hover["result"], Value::Null, "{hover}");
    }
    assert_eq!(
        d.references(&mut c, "Circle", 0, true),
        vec![
            d.range("Circle", 0),
            d.range("Circle", 1),
            d.range("Circle", 2)
        ]
    );
    assert_eq!(
        d.rename(&mut c, "shape", 0, "sh").unwrap(),
        src.replace("Some(shape)", "Some(sh)")
            .replace("= shape {", "= sh {")
    );
    assert_eq!(
        d.rename(&mut c, "Rect", 0, "Square").unwrap(),
        src.replace("Rect", "Square")
    );
    stop(child, c);
}
