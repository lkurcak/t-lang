//! Deterministic families for `for` over collections
//! (docs/73-for-over-collections.md): every kind of head, by reference, by
//! `&mut` and by value, generic and not, with bodies that break, skip and
//! call closures, against what the host computes; what each head's items
//! are known to be, read and written through; and that a `for` over a
//! list by `&mut` keeps its length. They're about what programs mean, so
//! they run on Linux alone.

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
const VALUES: [i64; 5] = [3, 1, 4, 1, 5];

/// A kind of head: what's set up before the loop, the head, and how the
/// loop's variable is read as an `i64`.
struct Head {
    name: &'static str,
    setup: &'static str,
    head: &'static str,
    read: &'static str,
}

fn heads() -> Vec<Head> {
    vec![
        Head {
            name: "list by reference",
            setup: "let l = list();",
            head: "&l",
            read: "*x",
        },
        Head {
            name: "list by mutable reference",
            setup: "let mut l = list();",
            head: "&mut l",
            read: "*x",
        },
        Head {
            name: "list by value",
            setup: "let l = list();",
            head: "l",
            read: "x",
        },
        Head {
            name: "list's iter",
            setup: "let l = list();",
            head: "l.iter()",
            read: "*x",
        },
        Head {
            name: "array by reference",
            setup: "let a = [3i64, 1, 4, 1, 5];",
            head: "&a",
            read: "*x",
        },
        Head {
            name: "array by mutable reference",
            setup: "let mut a = [3i64, 1, 4, 1, 5];",
            head: "&mut a",
            read: "*x",
        },
        Head {
            name: "array by value",
            setup: "let a = [3i64, 1, 4, 1, 5];",
            head: "a",
            read: "x",
        },
        Head {
            name: "slice",
            setup: "let a = [3i64, 1, 4, 1, 5]; let s: &[i64] = &a;",
            head: "s",
            read: "*x",
        },
        Head {
            name: "mutable slice",
            setup: "let mut a = [3i64, 1, 4, 1, 5]; let s: &mut [i64] = &mut a;",
            head: "s",
            read: "*x",
        },
        Head {
            name: "opaque iterator",
            setup: "",
            head: "values()",
            read: "x",
        },
        Head {
            name: "iterator by mutable reference",
            setup: "let mut it = values();",
            head: "&mut it",
            read: "x",
        },
        Head {
            name: "generic iterator",
            setup: "",
            head: "through(values())",
            read: "x",
        },
        Head {
            name: "generic collection",
            setup: "let l = list();",
            head: "into(&l)",
            read: "*x",
        },
    ]
}

/// A loop body, with what it prints for `VALUES`.
struct Body {
    name: &'static str,
    code: &'static str,
    expected: fn() -> i64,
}

fn bodies() -> Vec<Body> {
    vec![
        Body {
            name: "sums",
            code: "n += v;",
            expected: || VALUES.iter().sum(),
        },
        Body {
            name: "breaks at 4",
            code: "if v == 4 { break; } n += v;",
            expected: || VALUES.iter().take_while(|v| **v != 4).sum(),
        },
        Body {
            name: "skips odd",
            code: "if v % 2 == 1 { continue; } n += v;",
            expected: || VALUES.iter().filter(|v| **v % 2 == 0).sum(),
        },
        Body {
            name: "calls a closure",
            code: "let twice = |y: i64| y * 2; n += twice(v);",
            expected: || VALUES.iter().map(|v| v * 2).sum(),
        },
        Body {
            name: "counts",
            code: "let _v = v; n += 1;",
            expected: || VALUES.len() as i64,
        },
    ]
}

/// The program each head and body run in, with what the heads use.
fn program(head: &Head, body: &Body) -> String {
    format!(
        "use alloc::list::List;

fn list() -> List<i64> with alloc {{
    let mut l = List::new();
    for v in [3i64, 1, 4, 1, 5] {{
        l.push(v);
    }}
    l
}}

fn values() -> impl Iterator<Item = i64, e = {{}}> + Discard {{ [3i64, 1, 4, 1, 5].into_iter() }}

fn through<I: Iterator>(it: I) -> I {{ it }}

fn into<C: IntoIterator>(c: C) -> C::IntoIter {{ c.into_iter() }}

fn main(os: std::Os) -> Result<(), std::io::Error> with {{alloc, panic}} {{
    let mut out = os.stdout;
    let mut n = 0i64;
    {setup}
    for x in {head} {{
        let v: i64 = {read};
        {code}
    }}
    out.line(&n)
}}
",
        setup = head.setup,
        head = head.head,
        read = head.read,
        code = body.code,
    )
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn every_head_goes_through_its_values() {
    for head in heads() {
        for body in bodies() {
            let src = program(&head, &body);
            let expected = format!("{}\n", (body.expected)());
            let checked = t_lang::check_source(&src);
            if let Err(e) = &checked.result {
                panic!(
                    "{} / {}:\n{src}\n{}",
                    head.name,
                    body.name,
                    e.render_in(&checked.sources)
                );
            }
            accepted(&src, &expected);
        }
    }
}

/// A head over values refined `t < 5`, and how an item is read and written.
struct Refined {
    setup: &'static str,
    head: &'static str,
    read: &'static str,
    write: Option<&'static str>,
}

fn refined() -> Vec<Refined> {
    vec![
        Refined {
            setup: "let mut l: List<{ t: usize | t < 5 }> = List::new(); l.push(1);",
            head: "&l",
            read: "*x",
            write: None,
        },
        Refined {
            setup: "let mut l: List<{ t: usize | t < 5 }> = List::new(); l.push(1);",
            head: "&mut l",
            read: "*x",
            write: Some("*x = 5;"),
        },
        Refined {
            setup: "let mut l: List<{ t: usize | t < 5 }> = List::new(); l.push(1);",
            head: "l",
            read: "x",
            write: None,
        },
        Refined {
            setup: "let mut a: [{ t: usize | t < 5 }; 2] = [1, 2];",
            head: "&a",
            read: "*x",
            write: None,
        },
        Refined {
            setup: "let mut a: [{ t: usize | t < 5 }; 2] = [1, 2];",
            head: "&mut a",
            read: "*x",
            write: Some("*x = 5;"),
        },
        Refined {
            setup: "let mut a: [{ t: usize | t < 5 }; 2] = [1, 2];",
            head: "a",
            read: "x",
            write: None,
        },
        Refined {
            setup: "let mut a: [{ t: usize | t < 5 }; 2] = [1, 2]; let s: &mut [{ t: usize | t < 5 }] = &mut a;",
            head: "s",
            read: "*x",
            write: Some("*x = 5;"),
        },
    ]
}

fn refined_program(r: &Refined, body: &str) -> String {
    format!(
        "#![allow(unused_mut)]
use alloc::list::List;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {{
    let mut out = os.stdout;
    let five = [10u8, 11, 12, 13, 14];
    {setup}
    for x in {head} {{
        {body}
    }}
    out.line(&five[0])
}}
",
        setup = r.setup,
        head = r.head,
        body = body,
    )
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn items_are_what_is_declared_behind_them() {
    for r in refined() {
        // What's read is known: it indexes an array of 5.
        let read = format!("let i: usize = {}; let _b = five[i];", r.read);
        accepted(&refined_program(&r, &read), "10\n");
        // What's written must be too.
        if let Some(write) = r.write {
            rejected(&refined_program(&r, write), "can't prove `t < 5`");
        }
    }
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn a_list_keeps_its_length_through_mutable_references() {
    let src = "use alloc::list::List;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: List<i64> = List::new();
    l.push(1);
    l.push(2);
    l.push(3);
    for x in &mut l {
        *x = 7;
    }
    out.line(&l[2])
}
";
    accepted(src, "7\n");
}
