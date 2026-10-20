//! `t fmt`. Every program in `tests/t/` and `examples/`, and the core
//! library's files in `core/`, must format to the same program
//! (`fmt::format` checks that itself, and fails otherwise), and formatting
//! must be idempotent: formatting the output again changes nothing.
//! `examples/` and `core/` must be formatted already; test programs may
//! keep their own layouts, since some of them test the parser on odd ones.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use t_lang::fmt::{FmtError, format};
use t_lang::lexer::{Tok, Token};

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
    for path in programs("tests/t")
        .into_iter()
        .chain(programs("examples"))
        .chain(programs("core"))
    {
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

/// One style whatever the input looks like: a program's tokens all on one
/// line, and each on a line of its own, format the same. (Without comments
/// or blank lines, which `t fmt` keeps from the input, and so without doc
/// comments that document nothing, which it keeps like comments. A doc
/// comment that documents something is part of the program, but it runs to
/// the end of its line, so one ends the line.)
#[test]
fn layout_does_not_depend_on_the_input() {
    let mut failures = Vec::new();
    let mut compared = 0;
    for path in programs("tests/t")
        .into_iter()
        .chain(programs("examples"))
        .chain(programs("core"))
    {
        let name = path.display().to_string();
        let src = fs::read_to_string(&path).unwrap();
        let Ok(toks) = t_lang::lexer::lex(&src) else {
            continue;
        };
        let Ok(program) = t_lang::parser::parse(toks.clone()) else {
            continue;
        };
        let stray = |t: &&Token| {
            let mut lines = program.stray_docs.iter().flat_map(|d| &d.lines);
            lines.any(|l| l.span == t.span)
        };
        let words: Vec<String> = toks
            .iter()
            .filter(|t| !stray(t))
            .map(|t| match t.tok {
                Tok::Doc(_) => format!("{}\n", &src[t.span.start..t.span.end]),
                _ => src[t.span.start..t.span.end].to_string(),
            })
            .collect();
        let (Ok(one_line), Ok(tall)) = (format(&words.join(" ")), format(&words.join("\n"))) else {
            continue;
        };
        compared += 1;
        if one_line != tall {
            failures.push(t_lang::fmt::diff(&name, &one_line, &tall));
        }
    }
    assert!(compared > 100, "only {compared} programs compared");
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn examples_are_formatted() {
    for path in programs("examples").into_iter().chain(programs("core")) {
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
    print(p.x);               // expect: 1
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
fn prints_doc_comments_from_the_ast() {
    let src = "\
///   Pairs.
///
/// With `b`.
struct P { /// The first.
a: int, b: int }
enum E { /// One.
A, B }
/// Adds.
// plain
fn add(p: P) -> int {
    /// documents nothing
    p.a + p.b
}
//// plain, not a doc
";
    assert_eq!(
        fmt(src),
        "\
///   Pairs.
///
/// With `b`.
struct P {
    /// The first.
    a: int,
    b: int,
}
enum E {
    /// One.
    A,
    B,
}
/// Adds.
// plain
fn add(p: P) -> int {
    /// documents nothing
    p.a + p.b
}
//// plain, not a doc
"
    );
}

/// A nested `fn` is printed like a top-level one, with its doc comment
/// and the comments in and around it, and blank lines around it are kept
/// or dropped as around any statement.
#[test]
fn prints_nested_functions_like_statements() {
    let src = "\
fn main() {


    /// Doubles.
    ///   Indented.
    // plain, between the doc and the `fn`
    fn double(x: int) -> int { x * 2 } // after it
    fn half<T>(x: int, f: fn(int) -> T) -> T {
        // inside
        f(x / 2)
    }



    let x = half(double(3), |n| n); fn id(x: int) -> int { x }
    print(id(x))


}
";
    assert_eq!(
        fmt(src),
        "\
fn main() {
    /// Doubles.
    ///   Indented.
    // plain, between the doc and the `fn`
    fn double(x: int) -> int { x * 2 } // after it
    fn half<T>(x: int, f: fn(int) -> T) -> T {
        // inside
        f(x / 2)
    }

    let x = half(double(3), |n| n);
    fn id(x: int) -> int { x }
    print(id(x))
}
"
    );
}

/// Local types too: from the AST, with their docs, and with the blank
/// lines of any statement around them.
#[test]
fn prints_local_types_like_statements() {
    let src = "\
fn main() {


    /// A point.
    // plain, between the doc and the `struct`
    struct P { /// The x.
    x: int, y: int } // after it
    enum Shape<T> {
        // inside
        Circle(T), Empty
    }



    let p = P { x: 1, y: 2 }; struct Q {}
    print(p.x)


}
";
    assert_eq!(
        fmt(src),
        "\
fn main() {
    /// A point.
    // plain, between the doc and the `struct`
    struct P {
        /// The x.
        x: int,
        y: int,
    } // after it
    enum Shape<T> {
        // inside
        Circle(T),
        Empty,
    }

    let p = P { x: 1, y: 2 };
    struct Q {}
    print(p.x)
}
"
    );
}

/// A `///` that documents nothing is kept like a comment, but a comment
/// that can't stay where it is moves to the next place one can go, and
/// before a field a `///` would document it. Inside a struct or enum, they
/// go to the end instead.
#[test]
fn docs_that_document_nothing_keep_documenting_nothing() {
    let src = "\
struct P /// before the brace
{ a: int /// before the comma
, b: int, /// after the comma, so it documents c
c: int }
enum E { A(int /// in a variant's fields
), /// documents B
B }
fn f(/// before a parameter
x: int) {}
";
    assert_eq!(
        fmt(src),
        "\
struct P {
    a: int,
    b: int,
    /// after the comma, so it documents c
    c: int,
    /// before the brace
    /// before the comma
}
enum E {
    A(int),
    /// documents B
    B,
    /// in a variant's fields
}
fn f(
    /// before a parameter
    x: int,
) {}
"
    );
}

/// The check before writing compares doc comments as part of the program,
/// so moving one to something else would be refused, not written.
#[test]
fn doc_comments_are_part_of_the_program() {
    let parse = |src: &str| t_lang::parser::parse(t_lang::lexer::lex(src).unwrap()).unwrap();
    let documented = parse("/// A.\nfn a() {}\nfn b() {}\n");
    let moved = parse("fn a() {}\n/// A.\nfn b() {}\n");
    assert_ne!(t_lang::fmt::shape(&documented), t_lang::fmt::shape(&moved));
    let nested = parse("fn a() {\n    /// B.\n    fn b() {}\n    fn c() {}\n}\n");
    let moved = parse("fn a() {\n    fn b() {}\n    /// B.\n    fn c() {}\n}\n");
    assert_ne!(t_lang::fmt::shape(&nested), t_lang::fmt::shape(&moved));
    let stray = parse("fn a() {\n    /// A.\n}\n");
    assert_ne!(
        t_lang::fmt::shape(&stray),
        t_lang::fmt::shape(&parse("fn a() {}\n"))
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

/// `fn main() { <body> }`, formatted, with the `fn main() {` and `}` lines
/// taken off and the rest unindented.
fn fmt_body(body: &str) -> String {
    let out = fmt(&format!("fn main() {{\n{body}\n}}\n"));
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.first(), Some(&"fn main() {"), "{out}");
    assert_eq!(lines.last(), Some(&"}"), "{out}");
    let body: Vec<&str> = lines[1..lines.len() - 1]
        .iter()
        .map(|l| l.strip_prefix("    ").unwrap_or(l))
        .collect();
    body.join("\n") + "\n"
}

#[test]
fn lines_break_after_100_columns() {
    // 100 columns exactly: stays on one line.
    let call = format!("f({}, 1);", "a".repeat(89));
    assert_eq!(4 + call.len(), 100);
    assert_eq!(fmt_body(&call), format!("{call}\n"));
    // 101: breaks, one argument per line, with a comma after the last.
    let call = format!("f({}, 12);", "a".repeat(89));
    assert_eq!(
        fmt_body(&call),
        format!("f(\n    {},\n    12,\n);\n", "a".repeat(89))
    );
}

#[test]
fn groups_break_outermost_first() {
    let src = "let p = Pair { first: compute(alpha, beta, gamma), second: compute(delta, epsilon, zeta, eta, theta) };";
    assert_eq!(
        fmt_body(src),
        "\
let p = Pair {
    first: compute(alpha, beta, gamma),
    second: compute(delta, epsilon, zeta, eta, theta),
};
"
    );
}

#[test]
fn signatures_and_declarations_break_like_lists() {
    let src = "\
struct Configuration { width_in_columns: int, height_in_rows: int, depth_in_layers: int, colour: bool }
enum E<A> { One(A), Two(int, int) }
fn combine<Alpha, Beta>(first_argument: Alpha, second_argument: Beta, third_argument: int) -> int { 1 }
fn combine<Alpha, Beta>(first_argument: Alpha, second_argument: Beta, third_argument: int) -> Option<Beta> { 1 }
";
    assert_eq!(
        fmt(src),
        "\
struct Configuration {
    width_in_columns: int,
    height_in_rows: int,
    depth_in_layers: int,
    colour: bool,
}
enum E<A> { One(A), Two(int, int) }
fn combine<Alpha, Beta>(first_argument: Alpha, second_argument: Beta, third_argument: int) -> int {
    1
}
fn combine<Alpha, Beta>(
    first_argument: Alpha,
    second_argument: Beta,
    third_argument: int,
) -> Option<Beta> { 1 }
"
    );
}

/// A long impl header breaks its `<..>`, keeping a short type on the
/// line, and its `where` clause goes on lines of its own, as a function's.
#[test]
fn impl_headers_break_their_generics_and_wheres() {
    let src = "\
impl<Alpha: Iterator, Beta: FnMut(&Alpha::Item) -> bool with p, p, Gamma: Discard> Iterator for Filtered<Alpha, Beta, Gamma> {}
impl<I: Iterator> Iterator for Second<I> where I::Item: Discard { type Item = I::Item; }
impl<I: Iterator> Tr for W<I> where I::Item: Discard, I::Item: Eq, I::Item: Copy, I::Item: Ord, I::Item: Tr {}
";
    assert_eq!(
        fmt(src),
        "\
impl<
    Alpha: Iterator,
    Beta: FnMut(&Alpha::Item) -> bool with p,
    p,
    Gamma: Discard,
> Iterator for Filtered<Alpha, Beta, Gamma> {}
impl<I: Iterator> Iterator for Second<I> where I::Item: Discard {
    type Item = I::Item;
}
impl<I: Iterator> Tr for W<I>
where
    I::Item: Discard,
    I::Item: Eq,
    I::Item: Copy,
    I::Item: Ord,
    I::Item: Tr,
{}
"
    );
}

#[test]
fn a_last_closure_or_block_argument_breaks_inside() {
    let src = "\
apply(first_argument, second_argument, |x: int| -> int { let y = x * 2; y + 1 });
print(match some_long_scrutinee { Shape::Circle(radius) => radius, Shape::Empty => 0 });
print(compute_something_long(first_argument, second_argument, third_argument, fourth_argument, fifth));
";
    assert_eq!(
        fmt_body(src),
        "\
apply(first_argument, second_argument, |x: int| -> int {
    let y = x * 2;
    y + 1
});
print(match some_long_scrutinee {
    Shape::Circle(radius) => radius,
    Shape::Empty => 0,
});
print(compute_something_long(
    first_argument,
    second_argument,
    third_argument,
    fourth_argument,
    fifth,
));
"
    );
}

#[test]
fn operators_break_before_each_in_a_run() {
    let src = "\
let total = width_in_columns + height_in_rows * depth_in_layers + width_in_columns - height_in_rows;
if total > 1000 && colour || width_in_columns == height_in_rows && depth_in_layers != 100000 { print(1) }
";
    assert_eq!(
        fmt_body(src),
        "\
let total = width_in_columns
    + height_in_rows * depth_in_layers
    + width_in_columns
    - height_in_rows;
if (total > 1000 && colour)
    || (width_in_columns == height_in_rows && depth_in_layers != 100000)
{
    print(1)
}
"
    );
}

#[test]
fn short_blocks_go_on_one_line() {
    let src = "\
let a = if c { 1 } else { 2 };
let b = if c { if d { 1 } else { 2 } } else { 3 };
let f = |x| { x + 1 };
let g = |x| { let y = x; y };
";
    assert_eq!(
        fmt_body(src),
        "\
let a = if c { 1 } else { 2 };
let b = if c {
    if d { 1 } else { 2 }
} else {
    3
};
let f = |x| { x + 1 };
let g = |x| {
    let y = x;
    y
};
"
    );
}

#[test]
fn comments_at_line_ends_line_up_in_runs() {
    // A line without one ends the run, so `long_name_here(1)`'s comment
    // only moves its neighbour's.
    let src = "\
print(1); // expect: 1
print(22);   // expect: 22
let x = 1;
long_name_here(1);  // a
f(); // b
";
    assert_eq!(
        fmt_body(src),
        "\
print(1);  // expect: 1
print(22); // expect: 22
let x = 1;
long_name_here(1); // a
f();               // b
"
    );
    // A line that would push a comment past 100 columns starts a new run.
    let long = format!("print({});", "x".repeat(70));
    let src = format!(
        "print(1); // expect: 1\n{long} // expect: 2\nprint(3); // a comment, with more to say\n"
    );
    assert_eq!(
        fmt_body(&src),
        format!(
            "print(1);{} // expect: 1\n{long} // expect: 2\nprint(3); // a comment, with more to say\n",
            " ".repeat(long.len() - 9)
        )
    );
}

#[test]
fn jumps_keep_what_they_take() {
    // What follows `return` or `break` runs as far right as it can, so as
    // an operand one needs parentheses, even with nothing after it:
    // `return - 1` would return `-1`.
    let src = "\
let a = loop { break (return) - 1; };
let b = 1 + (return 2);
let c = (break) || true;
loop { continue; }
'outer:   while c { loop { continue   'outer; } break 'outer; }
return a+b;
";
    assert_eq!(
        fmt_body(src),
        "\
let a = loop {
    break (return) - 1;
};
let b = 1 + (return 2);
let c = (break) || true;
loop {
    continue;
}
'outer: while c {
    loop {
        continue 'outer;
    }
    break 'outer;
}
return a + b;
"
    );
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
