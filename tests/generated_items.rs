//! Deterministic families for what an iterator gives
//! (docs/82-item-invariants.md): a range through every chain of up to two
//! adapters, needing bounds the host works out from what each adapter
//! keeps, accepted exactly when the range's bounds (moved by a `map`) are
//! within them; and an iterator changed in a loop every way, which keeps
//! what it gives exactly when each change is a call of its protocol, or of
//! a function generic in it given no other. Each accepted program runs in
//! the interpreter, whose oracle checks every value given to `need`, and
//! must print what the host computes. They're about what programs mean,
//! so they run on Linux alone.

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

/// An adapter: its text, after the iterator; whether what it's given
/// must be double-ended, and whether what it gives is; how much it moves
/// the values; and what it does to them, on the host.
struct Adapter {
    text: &'static str,
    needs_back: bool,
    keeps_back: bool,
    shift: usize,
    run: fn(Vec<usize>) -> Vec<usize>,
}

fn adapters() -> Vec<Adapter> {
    vec![
        Adapter {
            text: ".rev()",
            needs_back: true,
            keeps_back: true,
            shift: 0,
            run: |v| v.into_iter().rev().collect(),
        },
        Adapter {
            text: ".skip(1)",
            needs_back: false,
            keeps_back: false,
            shift: 0,
            run: |v| v.into_iter().skip(1).collect(),
        },
        Adapter {
            text: ".filter(|x| *x % 2 == 0)",
            needs_back: false,
            keeps_back: false,
            shift: 0,
            run: |v| v.into_iter().filter(|x| x % 2 == 0).collect(),
        },
        Adapter {
            text: ".map(|x| x + 2)",
            needs_back: false,
            keeps_back: true,
            shift: 2,
            run: |v| v.into_iter().map(|x| x + 2).collect(),
        },
        Adapter {
            text: ".enumerate().map(|(_i, x)| x)",
            needs_back: false,
            keeps_back: false,
            shift: 0,
            run: |v| v,
        },
        Adapter {
            text: ".zip(0..100usize).map(|(x, _y)| x)",
            needs_back: false,
            keeps_back: false,
            shift: 0,
            run: |v| v,
        },
    ]
}

/// Every chain of up to two adapters that type-checks: `rev` only of what
/// can give from the back.
fn chains() -> Vec<Vec<usize>> {
    let n = adapters().len();
    let mut out = vec![Vec::new()];
    for a in 0..n {
        out.push(vec![a]);
        for b in 0..n {
            out.push(vec![a, b]);
        }
    }
    let ads = adapters();
    out.retain(|chain| {
        let mut back = true;
        for &k in chain {
            if ads[k].needs_back && !back {
                return false;
            }
            back = back && ads[k].keeps_back;
        }
        true
    });
    out
}

/// A program that goes through `lo..hi` by `chain`, giving each value to
/// a `need` of `v >= low && v < high`, printing it.
fn chain_program(lo: usize, hi: usize, chain: &[usize], low: usize, high: usize) -> String {
    let ads = adapters();
    let text: String = chain.iter().map(|&k| ads[k].text).collect();
    format!(
        "fn need(v: {{ v: usize | v >= {low} && v < {high} }}) -> usize {{ v }}

fn main(os: std::Os) -> Result<(), std::io::Error> {{
    let mut out = os.stdout;
    for v in ({lo}..{hi}){text} {{
        out.line(&need(v))?;
    }}
    Ok(())
}}
"
    )
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore)]
fn ranges_through_adapters() {
    let ads = adapters();
    let mut count = 0;
    for (lo, hi) in [(0, 4), (2, 6), (3, 3)] {
        for chain in chains() {
            let shift: usize = chain.iter().map(|&k| ads[k].shift).sum();
            let (low, high) = (lo + shift, hi + shift);
            let mut values: Vec<usize> = (lo..hi).collect();
            for &k in &chain {
                values = (ads[k].run)(values);
            }
            let output: String = values.iter().map(|v| format!("{v}\n")).collect();
            // The bounds it gives, and one end each pulled in: what's known
            // is the range's bounds, so those hold exactly when there are
            // no values at all.
            for (l, h) in [(low, high), (low + 1, high), (low, high.max(1) - 1)] {
                let src = chain_program(lo, hi, &chain, l, h);
                if (l, h) == (low, high) || lo == hi {
                    accepted(&src, &output);
                } else {
                    rejected(&src, "for argument `v` of `need`");
                }
                count += 1;
            }
        }
    }
    assert_eq!(count, 3 * chains().len() * 3);
}

/// A change to the iterator in the loop: its text, and whether what the
/// iterator gives is kept through it.
const CHANGES: [(&str, bool); 9] = [
    ("", true),
    ("let _ = it.next();", true),
    ("let _ = it.any(|x| x > 5);", true),
    ("step(&mut it);", true),
    ("if v == 3 {\n            let _ = it.next();\n        }", true),
    ("it = (0..100).rev();", false),
    ("widen(&mut it);", false),
    ("if v == 3 {\n            widen(&mut it);\n        }", false),
    ("swap_in(&mut it, (0..100).rev());", false),
];

/// A program changing `(0..8).rev()` in its loop by `change`.
fn change_program(change: &str) -> String {
    format!(
        "use core::iter::Rev;
use core::ops::Range;

fn need(v: {{ v: usize | v < 8 }}) -> usize {{ v }}

fn widen(it: &mut Rev<Range<usize>>) {{ *it = (0..100).rev(); }}

fn step<I: Iterator>(it: &mut I) with I::e where I::Item: Discard {{ let _ = it.next(); }}

fn swap_in<I: Discard>(it: &mut I, other: I) {{ *it = other; }}

#[allow(unused_assignments)]
fn main(os: std::Os) -> Result<(), std::io::Error> {{
    let mut out = os.stdout;
    let mut it = (0..8).rev();
    while let Some(v) = it.next() {{
        out.line(&need(v))?;
        {change}
    }}
    Ok(())
}}
"
    )
}

/// What the program changing its iterator by `change` prints.
fn change_output(change: &str) -> String {
    let mut out = String::new();
    let mut it = (0..8usize).rev();
    while let Some(v) = it.next() {
        out += &format!("{v}\n");
        match change {
            "let _ = it.next();" | "step(&mut it);" => {
                it.next();
            }
            "let _ = it.any(|x| x > 5);" => {
                let _ = it.any(|x| x > 5);
            }
            c if c.starts_with("if v == 3") => {
                if v == 3 {
                    it.next();
                }
            }
            _ => {}
        }
    }
    out
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore)]
fn iterators_changed_in_loops() {
    for (change, kept) in CHANGES {
        let src = change_program(change);
        match kept {
            true => accepted(&src, &change_output(change)),
            false => rejected(&src, "for argument `v` of `need`"),
        }
    }
}
