//! Deterministic families for counting in loops (docs/84-counting.md):
//! every iterator that says what it has left, as a `for` names it, with
//! bodies that count, index another array, skip, break, continue, give the
//! iterator a new value or lend it out, under an invariant that keeps a
//! count or an index with what's left. Each is refused for the loop's
//! invariant, or accepted and run by the interpreter, which checks every
//! refinement and every invariant at every head, against what the host
//! computes. They're about what programs mean, so they run on Linux alone.

use t_lang::error::Phase;

/// Checks `src` and runs it: it must print `output`.
fn accepted(src: &str, output: &str) {
    let checked = t_lang::check_source(src);
    let program = checked
        .result
        .unwrap_or_else(|e| panic!("rejected:\n{src}\n{}", e.render_in(&checked.sources)));
    let mut terminal = t_lang::interp::Captured::default();
    let result = t_lang::interp::run(&program.lower(), &mut terminal)
        .unwrap_or_else(|e| panic!("{src}\n{}", e.render_in(&checked.sources)));
    assert!(
        matches!(result, t_lang::interp::Value::Variant(_, ref v, _) if v == "Ok"),
        "{src}"
    );
    assert_eq!(String::from_utf8(terminal.out).unwrap(), output, "{src}");
}

/// Checks `src`: it's refused, with `message` in the error.
fn rejected(src: &str, message: &str) {
    let checked = t_lang::check_source(src);
    let error = checked
        .result
        .expect_err(&format!("accepted an invalid program:\n{src}"));
    assert_eq!(error.phase, Phase::Type, "{src}");
    assert!(
        error.message.contains(message),
        "{src}\n{}",
        error.render_in(&checked.sources)
    );
}

/// The values every head goes through.
const VALUES: [u64; 5] = [3, 1, 4, 1, 5];

/// What a body looks like: `{read}` for the value, `{empty}` for an
/// iterator of the head's type with nothing left, if it has one.
struct Body {
    name: &'static str,
    /// The invariant, of `n`, `i` and `it`, with `{len}` for how many
    /// values the head has.
    invariant: &'static str,
    code: &'static str,
    /// What it prints, or `Err` with what the refusal says.
    expected: fn() -> Result<u64, &'static str>,
    /// Does it need `{empty}`?
    empty: bool,
}

/// The loop's invariant, at the end of its body or at a `continue`.
const INVARIANT: &str = "for the loop's invariant";

fn bodies() -> Vec<Body> {
    vec![
        Body {
            name: "counts",
            invariant: "n + it.len() <= {len}",
            code: "if {read} > 1 { n = n + 1; }",
            expected: || Ok(VALUES.iter().filter(|v| **v > 1).count() as u64),
            empty: false,
        },
        Body {
            name: "counts with an index",
            invariant: "i + it.len() == {len} && n <= i",
            code: "if b[i] > {read} { n = n + 1; } i = i + 1;",
            expected: || Ok(5),
            empty: false,
        },
        Body {
            name: "counts and skips after 4",
            invariant: "n + it.len() <= {len}",
            code: "if {read} == 4 { let _ = it.next(); } n = n + 1;",
            expected: || Ok(4),
            empty: false,
        },
        Body {
            name: "indexes and skips after 4",
            invariant: "i + it.len() == {len} && n <= i",
            code: "if b[i] > {read} { n = n + 1; } if {read} == 4 { let _ = it.next(); } i = i + 1;",
            expected: || Err(INVARIANT),
            empty: false,
        },
        Body {
            name: "breaks at 4",
            invariant: "n + it.len() <= {len}",
            code: "if {read} == 4 { break; } n = n + 1;",
            expected: || Ok(2),
            empty: false,
        },
        Body {
            name: "continues at 1",
            invariant: "n + it.len() <= {len}",
            code: "if {read} == 1 { continue; } n = n + 1;",
            expected: || Ok(VALUES.iter().filter(|v| **v != 1).count() as u64),
            empty: false,
        },
        Body {
            name: "counts with no room",
            invariant: "n <= {len}",
            code: "n = n + 1;",
            expected: || Err(INVARIANT),
            empty: false,
        },
        Body {
            name: "lends it out",
            invariant: "n + it.len() <= {len}",
            code: "n = n + 1; drain(&mut it);",
            expected: || Err(INVARIANT),
            empty: false,
        },
        Body {
            name: "empties it",
            invariant: "n + it.len() <= {len}",
            code: "n = n + 1; if {read} == 4 { it = {empty}; }",
            expected: || Ok(3),
            empty: true,
        },
        Body {
            name: "starts it again",
            invariant: "n + it.len() <= {len}",
            code: "n = n + 1; if {read} == 4 { it = {again}; }",
            expected: || Err(INVARIANT),
            empty: true,
        },
    ]
}

/// A head: what's set up, the head, how its value is read as a `u64`, how
/// many values it has (`{len}` in the invariant), an iterator of its
/// type with none left and one starting again, if it has them.
struct Head {
    name: &'static str,
    setup: &'static str,
    head: &'static str,
    read: &'static str,
    len: &'static str,
    empty: Option<&'static str>,
    again: Option<&'static str>,
}

fn heads() -> Vec<Head> {
    vec![
        Head {
            name: "array by reference",
            setup: "let a = values();",
            head: "&a",
            read: "*x",
            len: "5",
            empty: Some("core::slice::empty::<u64>().iter()"),
            again: Some("a.iter()"),
        },
        Head {
            name: "slice",
            setup: "let a = values(); let s: &[u64] = &a;",
            head: "s",
            read: "*x",
            len: "s.len()",
            empty: Some("core::slice::empty::<u64>().iter()"),
            again: Some("s.iter()"),
        },
        Head {
            name: "slice's iter",
            setup: "let a = values(); let s: &[u64] = &a;",
            head: "s.iter()",
            read: "*x",
            len: "s.len()",
            empty: Some("core::slice::empty::<u64>().iter()"),
            again: Some("s.iter()"),
        },
        Head {
            name: "slice's copies",
            setup: "let a = values(); let s: &[u64] = &a;",
            head: "s.iter().copied()",
            read: "x",
            len: "s.len()",
            empty: Some("core::slice::empty::<u64>().iter().copied()"),
            again: Some("s.iter().copied()"),
        },
        Head {
            name: "array by value",
            setup: "let a = values();",
            head: "a",
            read: "x",
            len: "5",
            empty: None,
            again: Some("a.into_iter()"),
        },
        Head {
            name: "array by mutable reference",
            setup: "let mut a = values();",
            head: "&mut a",
            read: "*x",
            len: "5",
            empty: Some("core::slice::empty_mut::<u64>().iter_mut()"),
            again: None,
        },
        Head {
            name: "list by reference",
            setup: "let l = list();",
            head: "&l",
            read: "*x",
            len: "l.len()",
            empty: Some("core::slice::empty::<u64>().iter()"),
            again: Some("l.iter()"),
        },
        Head {
            name: "list's iter",
            setup: "let l = list();",
            head: "l.iter()",
            read: "*x",
            len: "l.len()",
            empty: Some("core::slice::empty::<u64>().iter()"),
            again: Some("l.iter()"),
        },
        Head {
            name: "list by mutable reference",
            setup: "let mut l = list(); let total = l.len();",
            head: "&mut l",
            read: "*x",
            len: "total",
            empty: Some("core::slice::empty_mut::<u64>().iter_mut()"),
            again: None,
        },
        Head {
            name: "list's iter_mut",
            setup: "let mut l = list(); let total = l.len();",
            head: "l.iter_mut()",
            read: "*x",
            len: "total",
            empty: Some("core::slice::empty_mut::<u64>().iter_mut()"),
            again: None,
        },
        Head {
            name: "list by value",
            setup: "let l = list(); let total = l.len();",
            head: "l",
            read: "x",
            len: "total",
            empty: None,
            again: None,
        },
    ]
}

/// The program a head and a body run in.
fn program(head: &Head, body: &Body, code: &str) -> String {
    let invariant = body.invariant.replace("{len}", head.len);
    format!(
        "use alloc::list::List;

fn values() -> [u64; 5] {{ [3, 1, 4, 1, 5] }}

fn list() -> {{ l: List<u64> | l.len() == 5 }} with alloc {{
    let mut l = List::new();
    l.push(3);
    l.push(1);
    l.push(4);
    l.push(1);
    l.push(5);
    l
}}

fn drain<I: Iterator>(it: &mut I) where I::Item: Discard {{
    while let Some(_x) = it.next() {{}}
}}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {{
    let mut out = os.stdout;
    let b: [u64; 5] = [10, 20, 30, 40, 50];
    let mut n: usize = 0;
    let mut i: usize = 0;
    {setup}
    for x in it: {head} invariant {invariant} {{
        {code}
    }}
    let _ = i;
    let _ = b;
    out.line(&n)
}}
",
        setup = head.setup,
        head = head.head,
    )
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn every_iterator_counts_what_it_has_left() {
    let mut ran = 0;
    for head in heads() {
        for body in bodies() {
            let code = body.code.replace("{read}", head.read);
            let code = match (body.empty, code.contains("{again}")) {
                (false, _) => code,
                (true, true) => match head.again {
                    Some(again) => code.replace("{again}", again),
                    None => continue,
                },
                (true, false) => match head.empty {
                    Some(empty) => code.replace("{empty}", empty),
                    None => continue,
                },
            };
            // Named in the program, which a failure prints.
            let src = format!(
                "// {} / {}\n{}",
                head.name,
                body.name,
                program(&head, &body, &code)
            );
            match (body.expected)() {
                Ok(n) => accepted(&src, &format!("{n}\n")),
                Err(message) => rejected(&src, message),
            }
            ran += 1;
        }
    }
    assert!(ran > 80, "{ran}");
}

/// A range's iterator says where it's up to, which a count is at most.
#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn ranges_count_up_to_where_they_are() {
    let invariant = "n <= it.start() && it.start() <= it.end() && it.end() == 5";
    let cases: [(&str, Result<u64, &str>); 7] = [
        ("if x > 1 { n = n + 1; }", Ok(3)),
        ("if x == 2 { let _ = it.next(); } n = n + 1;", Ok(4)),
        ("if x == 2 { break; } n = n + 1;", Ok(2)),
        ("if x == 1 { continue; } n = n + 1;", Ok(4)),
        ("n = n + 1; if x == 2 { it = 5..5; }", Ok(3)),
        ("n = n + 1; if x == 2 { it = 0..5; }", Err(INVARIANT)),
        ("n = n + 1; if x == 2 { it = 4..9; }", Err(INVARIANT)),
    ];
    for (code, expected) in cases {
        let src = format!(
            "fn main(os: std::Os) -> Result<(), std::io::Error> {{
    let mut out = os.stdout;
    let mut n: u8 = 0;
    for x in it: 0..5u8 invariant {invariant} {{
        {code}
    }}
    out.line(&n)
}}
"
        );
        match expected {
            Ok(n) => accepted(&src, &format!("{n}\n")),
            Err(message) => rejected(&src, message),
        }
    }
}
