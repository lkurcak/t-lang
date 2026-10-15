//! `t fmt`. Every program in `tests/t/` and `examples/` must format to the
//! same program (`fmt::format` checks that itself, and fails otherwise), and
//! formatting must be idempotent: formatting the output again changes
//! nothing. `examples/` must be formatted already; test programs may keep
//! their own layouts, since some of them test the parser on odd ones.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use t_lang::fmt::{FmtError, format};

/// Every `*.t` file in `dir`, in order.
fn programs(dir: &str) -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
    let mut paths: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "t"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no programs in {}", dir.display());
    paths
}

#[test]
fn formatting_keeps_the_program_and_is_idempotent() {
    let mut failures = Vec::new();
    let mut formatted = 0;
    for path in programs("tests/t").into_iter().chain(programs("examples")) {
        let name = path.display().to_string();
        let src = fs::read_to_string(&path).unwrap();
        let once = match format(&src) {
            Ok(once) => once,
            // Tests of syntax errors have nothing to format.
            Err(FmtError::Syntax(_)) => continue,
            Err(e) => {
                failures.push(e.render(&name, &src));
                continue;
            }
        };
        formatted += 1;
        match format(&once) {
            Ok(twice) if twice == once => {}
            Ok(twice) => failures.push(format!(
                "{name}: formatting twice differs from once:\n{}",
                t_lang::fmt::diff(&name, &once, &twice)
            )),
            Err(e) => failures.push(e.render(&name, &once)),
        }
    }
    assert!(formatted > 100, "only {formatted} programs formatted");
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn examples_are_formatted() {
    for path in programs("examples") {
        let name = path.display().to_string();
        let src = fs::read_to_string(&path).unwrap();
        let formatted = format(&src).unwrap_or_else(|e| panic!("{}", e.render(&name, &src)));
        assert!(
            formatted == src,
            "{name} isn't formatted (run `t fmt {name}`):\n{}",
            t_lang::fmt::diff(&name, &src, &formatted)
        );
    }
}

/// Formats `src`, and checks that the result is already formatted.
fn fmt(src: &str) -> String {
    let out = format(src).unwrap_or_else(|e| panic!("{}", e.render("test.t", src)));
    assert_eq!(format(&out).unwrap(), out, "not idempotent");
    out
}

#[test]
fn keeps_comments() {
    let src = "\
// Leading, at the top.
struct P { x: int, // the x
  y: int }
fn main() { // after the brace
    // before a statement
    let p = P { x: 1, y: 2 };   // trailing
    print(p.x);  // expect: 1
                 // expect: 2
    // before the tail
    p.y
    // dangling at the end
}
// at the end
";
    assert_eq!(
        fmt(src),
        "\
// Leading, at the top.
struct P {
    x: int, // the x
    y: int,
}
fn main() { // after the brace
    // before a statement
    let p = P { x: 1, y: 2 }; // trailing
    print(p.x); // expect: 1
                // expect: 2
    // before the tail
    p.y
    // dangling at the end
}
// at the end
"
    );
}

#[test]
fn comments_inside_lists_break_them() {
    let src = "fn main() { f(1, // one\n 2); g(\n// lone\n) }\n";
    assert_eq!(
        fmt(src),
        "\
fn main() {
    f(
        1, // one
        2,
    );
    g(
        // lone
    )
}
"
    );
}

#[test]
fn a_comment_inside_an_expression_moves_to_the_next_line() {
    // There's no line of its own to keep it on, so it goes after the
    // statement it was in, rather than be lost.
    let src = "fn main() {\n    let x = 1 + // one\n        2;\n    x\n}\n";
    assert_eq!(
        fmt(src),
        "fn main() {\n    let x = 1 + 2;\n    // one\n    x\n}\n"
    );
}

#[test]
fn keeps_single_blank_lines() {
    let src = "\n\n\nfn f() {}\nfn g() {}\n\n\n\nfn main() {\n\n    f();\n\n\n    g();\n    // c\n\n    f()\n\n}\n\n\n";
    assert_eq!(
        fmt(src),
        "fn f() {}\nfn g() {}\n\nfn main() {\n    f();\n\n    g();\n    // c\n\n    f()\n}\n"
    );
}

#[test]
fn keeps_the_order_of_items() {
    let src = "fn main() { f(P { x: 1 }) }\nstruct P { x: int }\nfn f(p: P) -> int { p.x }\n";
    let out = fmt(src);
    let main = out.find("fn main").unwrap();
    let p = out.find("struct P").unwrap();
    let f = out.find("fn f").unwrap();
    assert!(main < p && p < f, "{out}");
}

#[test]
fn refuses_what_does_not_parse() {
    assert!(matches!(
        format("fn main() { let }"),
        Err(FmtError::Syntax(_))
    ));
}

#[test]
fn cli_rewrites_or_checks() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fmt_cli");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("messy.t");
    let messy = "fn main()   {print(1)}\n";
    fs::write(&path, messy).unwrap();
    let t = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_t"))
            .arg("fmt")
            .args(args)
            .arg(&path)
            .output()
            .unwrap()
    };

    let check = t(&["--check"]);
    assert_eq!(check.status.code(), Some(1));
    let diff = String::from_utf8(check.stdout).unwrap();
    assert!(diff.contains("-fn main()   {print(1)}"), "{diff}");
    assert!(diff.contains("+fn main() {"), "{diff}");
    assert_eq!(fs::read_to_string(&path).unwrap(), messy, "--check wrote");

    assert!(t(&[]).status.success());
    let formatted = fs::read_to_string(&path).unwrap();
    assert_eq!(formatted, format(messy).unwrap());

    let check = t(&["--check"]);
    assert!(check.status.success());
    assert!(check.stdout.is_empty());

    fs::write(&path, "fn main() {").unwrap();
    assert_eq!(t(&[]).status.code(), Some(1));
    assert_eq!(fs::read_to_string(&path).unwrap(), "fn main() {");
}
