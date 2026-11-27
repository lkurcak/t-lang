//! A deterministic family for values that may be dropped where a
//! refinement holds (docs/80-discard-when.md): a list of `n` values, made
//! and used up in one function, in one of many ways, of a type that isn't
//! `Discard` (`Fd`) and of one that is (`i64`). A program of `Fd`s is
//! accepted exactly when the checker can know every value is used up on
//! every path, which the host works out from `n` and the way; one of
//! `i64`s always is. Each accepted program runs, in the interpreter,
//! whose oracle stops it if a list that isn't empty is dropped by the impl
//! for empty ones, and must print what the host computes. They're about
//! what programs mean, so they run on Linux alone.

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

/// A way to use up `l`, a list of `n` values, adding what it uses up to
/// `total`, as the body of a function given `l` and `out`: `uses(n)` is
/// how many of them it uses up, from the back if `pops`, else the front.
struct Way {
    name: &'static str,
    body: &'static str,
    uses: fn(usize) -> usize,
    pops: bool,
}

fn ways() -> Vec<Way> {
    vec![
        Way {
            name: "for to its end",
            body: "for x in l {\n        total = total + take(x);\n    }",
            uses: |n| n,
            pops: false,
        },
        Way {
            name: "for that skips with continue",
            body: "for x in l {\n        let v = take(x);\n        if v == 2 {\n            continue;\n        }\n        total = total + v;\n    }",
            uses: |n| n,
            pops: false,
        },
        Way {
            name: "for that breaks",
            body: "for x in l {\n        total = total + take(x);\n        if total > 0 {\n            break;\n        }\n    }",
            uses: |n| n.min(1),
            pops: false,
        },
        Way {
            name: "for that returns",
            body: "for x in l {\n        return take(x);\n    }",
            uses: |n| n.min(1),
            pops: false,
        },
        Way {
            name: "while let pop",
            body: "while let Some(x) = l.pop() {\n        total = total + take(x);\n    }",
            uses: |n| n,
            pops: true,
        },
        Way {
            name: "one pop",
            body: "if let Some(x) = l.pop() {\n        total = total + take(x);\n    }",
            uses: |n| n.min(1),
            pops: true,
        },
        Way {
            name: "two pops",
            body: "if let Some(x) = l.pop() {\n        total = total + take(x);\n    }\n    if let Some(x) = l.pop() {\n        total = total + take(x);\n    }",
            uses: |n| n.min(2),
            pops: true,
        },
        Way {
            name: "nothing",
            body: "",
            uses: |_| 0,
            pops: false,
        },
        Way {
            name: "its iterator, dropped",
            body: "let _it = l.into_iter();",
            uses: |_| 0,
            pops: false,
        },
        Way {
            name: "its iterator, part by while let, the rest by for",
            body: "let mut it = l.into_iter();\n    while let Some(x) = it.next() {\n        total = total + take(x);\n        if total > 0 {\n            break;\n        }\n    }\n    for x in it {\n        total = total + take(x);\n    }",
            uses: |n| n,
            pops: false,
        },
        Way {
            name: "free_with if not empty",
            body: "if l.is_empty() {\n        return total;\n    }\n    l.free_with(|x| total = total + take(x));",
            uses: |n| n,
            pops: false,
        },
    ]
}

/// The values a list of `n` holds, front to back.
fn values(n: usize) -> Vec<i64> {
    (1..=n as i64).collect()
}

/// A program that makes a list of `n` values of `elem`, `Fd` or `i64`, and
/// uses it up `way`, in the same function, printing what it used up.
fn program(n: usize, way: &Way, elem: &str) -> String {
    let (wrap, take) = match elem {
        "Fd" => ("Fd { n: v }", "let Fd { n } = x;\n    n"),
        _ => ("v", "x"),
    };
    let mut pushes = String::new();
    for v in values(n) {
        pushes += &format!("    l.push(make({v}));\n");
    }
    format!(
        "use alloc::list::List;

#[allow(dead_code)]
struct Fd {{ n: i64 }}

fn make(v: i64) -> {elem} {{ {wrap} }}

fn take(x: {elem}) -> i64 {{
    {take}
}}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code)]
fn used() -> i64 with {{alloc, panic}} {{
    let mut total = 0;
    let mut l: List<{elem}> = List::new();
{pushes}    {body}
    total
}}

fn main(os: std::Os) -> Result<(), std::io::Error> with {{alloc, panic}} {{
    let mut out = os.stdout;
    out.line(&used())
}}
",
        body = way.body,
    )
}

/// What `used` gives for a list of `n` used up `way`.
fn expected(n: usize, way: &Way) -> i64 {
    let vs = values(n);
    let k = (way.uses)(n);
    let used: Vec<i64> = match way.pops {
        true => vs.iter().rev().take(k).copied().collect(),
        false => vs.iter().take(k).copied().collect(),
    };
    match way.name {
        "for that skips with continue" => used.iter().filter(|v| **v != 2).sum(),
        "for that returns" => used.first().copied().unwrap_or(0),
        _ => used.iter().sum(),
    }
}

/// Would the program of `Fd`s leave a value unused on some path the
/// checker can't rule out, and what it says: a `for` that may stop early,
/// after its first value, of more than one, as a loop forgets what it's
/// lent but what its iterator's own signatures bound (an iterator's length
/// never grows, so one of one value is empty after it, and one of none
/// never gives one: docs/82-item-invariants.md); or a list or iterator
/// that may hold values where it's dropped, which it knows from `n`.
fn leaks(n: usize, way: &Way) -> Option<&'static str> {
    match way.name {
        "for that breaks" | "for that returns" if n > 1 => {
            Some("the `for` may stop before its iterator")
        }
        "nothing" | "its iterator, dropped" | "one pop" | "two pops" if (way.uses)(n) < n => {
            Some("is dropped here")
        }
        _ => None,
    }
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore)]
fn lists_used_up_every_way() {
    let mut count = 0;
    for n in [0, 1, 2, 3] {
        for way in ways() {
            for elem in ["Fd", "i64"] {
                let src = program(n, &way, elem);
                let leak = leaks(n, &way).filter(|_| elem == "Fd");
                match leak {
                    Some(message) => rejected(&src, message),
                    None => accepted(&src, &format!("{}\n", expected(n, &way))),
                }
                count += 1;
            }
        }
    }
    assert_eq!(count, 4 * ways().len() * 2);
}
