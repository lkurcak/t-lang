//! The tree-sitter grammar in `editors/tree-sitter-t` must parse every
//! program that T's own parser does, and its highlight query must work with
//! it.

use std::fs;
use std::path::{Path, PathBuf};
use tree_sitter::{Node, Parser, Query, QueryCursor, StreamingIterator};

/// Every `.t` file in `tests/t` and `examples`, and the directories in them.
fn programs() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut dirs = vec![root.join("tests/t"), root.join("examples")];
    let mut paths = Vec::new();
    while let Some(dir) = dirs.pop() {
        for p in fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
        {
            if p.is_dir() {
                dirs.push(p);
            } else if p.extension().is_some_and(|ext| ext == "t") {
                paths.push(p);
            }
        }
    }
    paths.sort();
    assert!(!paths.is_empty());
    paths
}

/// Does T's own parser accept `src`? Programs that test syntax errors don't.
fn parses(src: &str) -> bool {
    t_lang::lexer::lex(src)
        .and_then(t_lang::parser::parse)
        .is_ok()
}

fn parser() -> Parser {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_t::LANGUAGE.into())
        .expect("the grammar's ABI is supported");
    parser
}

/// The first `ERROR` or `MISSING` node in `node`, if there is one.
fn first_error(node: Node) -> Option<Node> {
    if node.is_error() || node.is_missing() {
        return Some(node);
    }
    if !node.has_error() {
        return None;
    }
    let mut cursor = node.walk();
    node.children(&mut cursor).find_map(first_error)
}

#[test]
fn parses_every_program() {
    let mut parser = parser();
    let mut failures = Vec::new();
    let mut checked = 0;
    for path in programs() {
        let src = fs::read_to_string(&path).unwrap();
        if !parses(&src) {
            continue;
        }
        checked += 1;
        let tree = parser.parse(&src, None).unwrap();
        if let Some(node) = first_error(tree.root_node()) {
            let pos = node.start_position();
            failures.push(format!(
                "{}:{}:{}: {} in {}",
                path.display(),
                pos.row + 1,
                pos.column + 1,
                if node.is_missing() {
                    "MISSING"
                } else {
                    "ERROR"
                },
                node.parent().map_or("the file", |p| p.kind()),
            ));
        }
    }
    assert!(checked > 0);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Helix and Neovim disagree on which of two captures of the same node wins,
/// so the query must never capture a node twice.
#[test]
fn highlights_capture_each_node_once() {
    let language = tree_sitter_t::LANGUAGE.into();
    let query = Query::new(&language, tree_sitter_t::HIGHLIGHTS_QUERY).unwrap();
    let mut parser = parser();
    let mut failures = Vec::new();
    for path in programs() {
        let src = fs::read_to_string(&path).unwrap();
        let tree = parser.parse(&src, None).unwrap();
        let mut seen = std::collections::HashMap::new();
        let mut cursor = QueryCursor::new();
        let mut captures = cursor.captures(&query, tree.root_node(), src.as_bytes());
        while let Some((m, i)) = captures.next() {
            let c = m.captures[*i];
            let name = query.capture_names()[c.index as usize];
            if let Some(prev) = seen.insert(c.node.id(), name)
                && prev != name
            {
                let pos = c.node.start_position();
                failures.push(format!(
                    "{}:{}:{}: `{}` is both @{prev} and @{name}",
                    path.display(),
                    pos.row + 1,
                    pos.column + 1,
                    c.node.utf8_text(src.as_bytes()).unwrap(),
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// `///` is a doc comment, highlighted as documentation; `//` and `////`
/// are plain comments.
#[test]
fn tells_doc_comments_from_comments() {
    let src = "/// doc\n///\n// plain\n//\n//// plain too\nfn main() {}\n";
    let tree = parser().parse(src, None).unwrap();
    let mut cursor = tree.root_node().walk();
    let kinds: Vec<(&str, &str)> = tree
        .root_node()
        .children(&mut cursor)
        .map(|n| (n.kind(), n.utf8_text(src.as_bytes()).unwrap()))
        .collect();
    assert_eq!(
        kinds[..5],
        [
            ("doc_comment", "/// doc"),
            ("doc_comment", "///"),
            ("line_comment", "// plain"),
            ("line_comment", "//"),
            ("line_comment", "//// plain too"),
        ]
    );
}

/// A `fn` in a block is a `function_item`, like one at the top level, and
/// its name is highlighted as a function.
#[test]
fn parses_nested_functions() {
    let src = "fn main() {\n    /// Doc.\n    fn helper<T>(x: T) -> T { x }\n    helper(1)\n}\n";
    let tree = parser().parse(src, None).unwrap();
    let root = tree.root_node();
    assert!(first_error(root).is_none(), "{}", root.to_sexp());
    let body = root.child(0).unwrap().child_by_field_name("body").unwrap();
    let mut cursor = body.walk();
    let kinds: Vec<&str> = body.named_children(&mut cursor).map(|n| n.kind()).collect();
    assert_eq!(kinds, ["doc_comment", "function_item", "call_expression"]);
    let nested = body.named_child(1).unwrap();
    let name = nested.child_by_field_name("name").unwrap();
    assert_eq!(name.utf8_text(src.as_bytes()).unwrap(), "helper");
    assert!(nested.child_by_field_name("type_parameters").is_some());

    let language = tree_sitter_t::LANGUAGE.into();
    let query = Query::new(&language, tree_sitter_t::HIGHLIGHTS_QUERY).unwrap();
    let mut cursor = QueryCursor::new();
    let mut captures = cursor.captures(&query, root, src.as_bytes());
    let mut function = None;
    while let Some((m, i)) = captures.next() {
        let c = m.captures[*i];
        if c.node.id() == name.id() {
            function = Some(query.capture_names()[c.index as usize]);
        }
    }
    assert_eq!(function, Some("function"));
}

#[test]
fn parses_local_types() {
    let src = "fn main() {\n    /// Doc.\n    struct P { x: int }\n    enum E<T> { A(T) }\n    P { x: 1 }.x\n}\n";
    let tree = parser().parse(src, None).unwrap();
    let root = tree.root_node();
    assert!(first_error(root).is_none(), "{}", root.to_sexp());
    let body = root.child(0).unwrap().child_by_field_name("body").unwrap();
    let mut cursor = body.walk();
    let kinds: Vec<&str> = body.named_children(&mut cursor).map(|n| n.kind()).collect();
    assert_eq!(
        kinds,
        [
            "doc_comment",
            "struct_item",
            "enum_item",
            "field_expression"
        ]
    );
    let name = body
        .named_child(1)
        .unwrap()
        .child_by_field_name("name")
        .unwrap();
    assert_eq!(name.utf8_text(src.as_bytes()).unwrap(), "P");
}

/// The `x` of a shorthand `P { x }`, in a literal or a pattern, is
/// highlighted as a variable, so not at all; the `y` of `y: 1` is a field.
#[test]
fn highlights_shorthand_fields_as_variables() {
    let src = "struct P { x: int, y: int }\nfn main() {\n    let x = 1;\n    let P { x, y: _ } = P { x, y: 1 };\n    x\n}\n";
    let tree = parser().parse(src, None).unwrap();
    let root = tree.root_node();
    assert!(first_error(root).is_none(), "{}", root.to_sexp());
    let language = tree_sitter_t::LANGUAGE.into();
    let query = Query::new(&language, tree_sitter_t::HIGHLIGHTS_QUERY).unwrap();
    let mut cursor = QueryCursor::new();
    let mut captures = cursor.captures(&query, root, src.as_bytes());
    let mut members = Vec::new();
    while let Some((m, i)) = captures.next() {
        let c = m.captures[*i];
        if query.capture_names()[c.index as usize] == "variable.other.member" {
            let line = c.node.start_position().row + 1;
            members.push((line, c.node.utf8_text(src.as_bytes()).unwrap()));
        }
    }
    assert_eq!(members, [(1, "x"), (1, "y"), (4, "y"), (4, "y")]);
}
