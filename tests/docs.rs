//! The T code in `README.md` and `docs/*.md`: every ```` ```rust ```` block
//! must parse and be formatted as `t fmt` would, and a whole program (one
//! with a `fn main`) must check and run, doing what its comments say, as in
//! `tests/t` (`// expect:`, `// error:`, `// warning:`; see
//! `shared/expect.rs`).
//!
//! Words after `rust`, separated by commas, say what else a block is.
//! GitHub only reads the first word, so they don't change how it shows.
//!
//! - `fragment`: statements, maybe ending in an expression, as in a
//!   function body. Checked as the body of a `fn main`, but not run: it may
//!   use names from elsewhere.
//! - `error`: a program shown for the error it gets. It must fail with the
//!   error in its `// error:` comment. An empty `fn main() {}` is added if
//!   it has none, so a lone function can show its error.
//! - `unformatted`: laid out other than as `t fmt` would, on purpose, such
//!   as the input in docs/fmt.md.
//!
//! Blocks in other languages, such as ```` ```text ````, aren't checked.

#[path = "shared/expect.rs"]
mod expect;

use std::fs;
use std::path::Path;
use t_lang::fmt::format;

const MARKERS: [&str; 3] = ["fragment", "error", "unformatted"];

struct Block {
    /// `docs/fmt.md:7`: where its opening fence is.
    name: String,
    path: String,
    /// The line its opening fence is on, counting from 1.
    line: usize,
    markers: Vec<String>,
    /// Without the indentation of the fence, if it's inside a list item.
    code: String,
}

impl Block {
    fn is(&self, marker: &str) -> bool {
        self.markers.iter().any(|m| m == marker)
    }
}

/// Every ```` ```rust ```` block in `path`.
fn blocks(path: &str) -> Vec<Block> {
    let src = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap();
    let mut out = Vec::new();
    let mut lines = src.lines().enumerate();
    while let Some((i, line)) = lines.next() {
        let indent = line.len() - line.trim_start().len();
        let Some(info) = line.trim_start().strip_prefix("```") else {
            continue;
        };
        let mut words = info.split(',').map(str::trim);
        let is_rust = words.next() == Some("rust");
        let markers: Vec<String> = words.map(String::from).collect();
        let mut code = String::new();
        for (_, line) in lines.by_ref() {
            if line.trim_start().starts_with("```") {
                break;
            }
            code.push_str(line.get(indent..).unwrap_or(""));
            code.push('\n');
        }
        if is_rust {
            out.push(Block {
                name: format!("{path}:{}", i + 1),
                path: path.to_string(),
                line: i + 1,
                markers,
                code,
            });
        }
    }
    out
}

/// `README.md` and every `docs/*.md`.
fn documents() -> Vec<String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs");
    let mut docs: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".md"))
        .map(|name| format!("docs/{name}"))
        .collect();
    docs.sort();
    docs.insert(0, "README.md".to_string());
    docs
}

/// A fragment as the body of `fn main`, indented as it would be there.
fn wrap(fragment: &str) -> String {
    let body: Vec<String> = fragment
        .lines()
        .map(|l| match l.is_empty() {
            true => String::new(),
            false => format!("    {l}"),
        })
        .collect();
    format!("fn main() {{\n{}\n}}\n", body.join("\n"))
}

/// The body of the `fn main` that `wrap` made, as it was before `wrap`.
fn unwrap(program: &str) -> Option<String> {
    // A lone expression goes on one line.
    if let Some(e) = program.strip_prefix("fn main() { ") {
        return Some(e.strip_suffix(" }\n")?.to_string() + "\n");
    }
    let body = program
        .strip_prefix("fn main() {\n")?
        .strip_suffix("\n}\n")?;
    let lines: Vec<&str> = body
        .lines()
        .map(|l| l.strip_prefix("    ").unwrap_or(l))
        .collect();
    Some(lines.join("\n") + "\n")
}

fn check(block: &Block) -> Result<(), String> {
    if let Some(m) = block
        .markers
        .iter()
        .find(|m| !MARKERS.contains(&m.as_str()))
    {
        return Err(format!("unknown marker `{m}`; known: {MARKERS:?}"));
    }
    let fragment = block.is("fragment");
    let src = match fragment {
        true => wrap(&block.code),
        false => block.code.clone(),
    };

    // Formatting parses it first.
    let formatted = match format(&src) {
        Ok(out) if fragment => unwrap(&out).ok_or_else(|| format!("formatted as\n{out}"))?,
        Ok(out) => out,
        Err(e) => return Err(e.render(&block.name, &src)),
    };
    let is_formatted = formatted == block.code;
    if is_formatted == block.is("unformatted") {
        return Err(match is_formatted {
            true => "is formatted, but marked `unformatted`".to_string(),
            false => format!(
                "isn't formatted:\n{}",
                t_lang::fmt::diff(&block.name, &block.code, &formatted)
            ),
        });
    }

    // Run with blank lines before it, so errors point at the line in the
    // document.
    let lines_before = block.line - usize::from(fragment);
    let src = "\n".repeat(lines_before) + &src;
    let has_main = src.contains("fn main(");
    if block.is("error") {
        if expect::Expectations::of(&src).error.is_none() {
            return Err("marked `error`, but has no `// error:` comment".to_string());
        }
        // After the code, so the error stays where it is.
        let src = match has_main {
            true => src,
            false => format!("{src}\nfn main() {{}}\n"),
        };
        expect::check(&block.path, &src)
    } else if has_main && !fragment {
        expect::check(&block.path, &src)
    } else {
        Ok(())
    }
}

#[test]
fn code_in_the_docs_parses_is_formatted_and_runs() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for doc in documents() {
        for block in blocks(&doc) {
            checked += 1;
            if let Err(e) = check(&block) {
                failures.push(format!("{}: {e}", block.name));
            }
        }
    }
    assert!(checked >= 24, "only {checked} blocks found");
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn fragments_unwrap_to_themselves() {
    let code = "let x = 1;\n\nif x > 0 {\n    print(x)\n}\n";
    assert_eq!(unwrap(&wrap(code)).as_deref(), Some(code));
    let one_line = format(&wrap("f(a, b)\n")).unwrap();
    assert_eq!(unwrap(&one_line).as_deref(), Some("f(a, b)\n"));
}

#[test]
fn finds_blocks_and_their_markers() {
    let blocks = blocks("docs/fmt.md");
    assert!(
        blocks
            .iter()
            .any(|b| b.is("unformatted") && !b.is("fragment"))
    );
    // Inside a list item, indented.
    let hug = blocks
        .iter()
        .find(|b| b.code.starts_with("apply("))
        .unwrap();
    assert!(
        hug.code
            .lines()
            .all(|l| !l.starts_with("  ") || l.starts_with("    "))
    );
}
