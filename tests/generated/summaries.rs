//! A generated family of programs that get a refined value back from a
//! closure or a function, written in each form the language has
//! (docs/78-callable-values.md): a closure written in the call, one made
//! before it (and moved, captured by another, kept in a field), a function
//! given by name, through a variable, as a method; given to a generic
//! function of each kind (`call`, `twice`, `unwrap_or_else`) or called
//! straight. What it gives keeps `v > 0` or breaks it.
//!
//! Every form is the same program, so each gets the same verdict: each
//! keeping the refinement is accepted and runs, each breaking it is refused
//! for the refinement, and none breaks it at run time (the interpreter's
//! oracle).
//!
//! They're about what programs mean, the same on every platform, so they run
//! on Linux alone.

use std::panic::catch_unwind;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Form {
    /// `|| give()`, written where it's used.
    Inline,
    /// `let f = || give();`, then `f`.
    Local,
    /// `let f0 = || give(); let f = f0;`, then `f`.
    Moved,
    /// `give`.
    Function,
    /// `let f = give;`, then `f`.
    FunctionLocal,
    /// `G::give`.
    Method,
    /// `let f0 = || give();`, then `|| f0()`.
    Captured,
    /// `let h = H { f: || give() };`, then `h.f`.
    Field,
    /// `|| n`, written where it's used.
    Literal,
    /// `let f = || { if c { return n; } 1 };`, then `f`: either return,
    /// as `c` is any `bool`.
    Branch,
    /// `let f = || { if c { return 1; } n };`: the other return first.
    BranchLast,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Taker {
    /// `call(f)`
    Call,
    /// `twice(f)`, both results read.
    Twice,
    /// `none.unwrap_or_else(f)`
    OrElse,
    /// `f()`
    Direct,
}

const FORMS: [Form; 11] = [
    Form::Inline,
    Form::Local,
    Form::Moved,
    Form::Function,
    Form::FunctionLocal,
    Form::Method,
    Form::Captured,
    Form::Field,
    Form::Literal,
    Form::Branch,
    Form::BranchLast,
];
const TAKERS: [Taker; 4] = [Taker::Call, Taker::Twice, Taker::OrElse, Taker::Direct];

/// A program whose closure or function gives `n` (0 breaks `v > 0`, 1
/// keeps it), in `form`, to `taker`, read into a refined `let`.
fn program(form: Form, taker: Taker, n: u32) -> String {
    let items = format!(
        "\
#![allow(dead_code)]

fn call<R, F: FnOnce() -> R + Discard>(f: F) -> R {{ f() }}

fn twice<R, F: FnMut() -> R + Discard>(mut f: F) -> (R, R) {{ (f(), f()) }}

fn give() -> {{ v: u32 | v == {n} }} {{ {n} }}

fn flag() -> bool {{ true }}

struct G {{}}

impl G {{
    fn give() -> {{ v: u32 | v == {n} }} {{ {n} }}
}}

#[derive(Discard)]
struct H<F> {{ f: F }}

"
    );
    // What's made first, and the value given or called.
    let (setup, value, called) = match form {
        Form::Inline => ("", "|| give()", "(|| give())()".to_string()),
        Form::Local => ("let f = || give();", "f", "f()".to_string()),
        Form::Moved => (
            "let f0 = || give();\n    let f = f0;",
            "f",
            "f()".to_string(),
        ),
        Form::Function => ("", "give", "give()".to_string()),
        Form::FunctionLocal => ("let f = give;", "f", "f()".to_string()),
        Form::Method => ("", "G::give", "G::give()".to_string()),
        Form::Captured => ("let f0 = || give();", "|| f0()", "(|| f0())()".to_string()),
        Form::Field => ("let h = H { f: || give() };", "h.f", "(h.f)()".to_string()),
        Form::Literal => ("", "", format!("(|| {n})()")),
        Form::Branch | Form::BranchLast => ("", "f", "f()".to_string()),
    };
    let literal = format!("|| {n}");
    let value = match form {
        Form::Literal => literal.as_str(),
        _ => value,
    };
    let branch = format!(
        "let f = || {{\n        if c {{\n            return {n};\n        }}\n        1\n    }};"
    );
    let branch_last = format!(
        "let f = || {{\n        if c {{\n            return 1;\n        }}\n        {n}\n    }};"
    );
    let setup = match form {
        Form::Branch => branch.as_str(),
        Form::BranchLast => branch_last.as_str(),
        _ => setup,
    };
    let read = match taker {
        Taker::Call => format!("let r: {{ v: u32 | v > 0 }} = call({value});"),
        Taker::Twice => format!(
            "let (a, b) = twice({value});\n    let r: {{ v: u32 | v > 0 }} = a;\n    let s: {{ v: u32 | v > 0 }} = b;\n    let _ = s;"
        ),
        Taker::OrElse => format!(
            "let none: Option<u32> = None;\n    let r: {{ v: u32 | v > 0 }} = none.unwrap_or_else({value});"
        ),
        Taker::Direct => format!("let r: {{ v: u32 | v > 0 }} = {called};"),
    };
    format!(
        "{items}fn main() with {{alloc, panic}} {{\n    let c = flag();\n    {setup}\n    {read}\n    let _ = (c, r);\n}}\n"
    )
}

#[derive(Debug, PartialEq)]
enum Verdict {
    Refused(String),
    Ran,
    Broke(String),
}

fn verdict(src: &str) -> Verdict {
    let checked = t_lang::check_source(src);
    let program = match checked.result {
        Ok(p) => p,
        Err(e) => return Verdict::Refused(e.render_in(&checked.sources)),
    };
    let lowered = program.lower();
    let run = catch_unwind(|| {
        let mut terminal = t_lang::interp::Captured::default();
        let _ = t_lang::interp::run(&lowered, &mut terminal);
    });
    match run {
        Ok(()) => Verdict::Ran,
        Err(panic) => {
            let why = (panic.downcast_ref::<String>().cloned())
                .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default();
            Verdict::Broke(why)
        }
    }
}

/// Each form keeping the refinement is accepted and runs; each breaking it
/// is refused, for the refinement; none breaks it at run time.
#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn every_form_gives_the_same() {
    let mut off = Vec::new();
    for n in [0, 1] {
        for form in FORMS {
            for taker in TAKERS {
                let label = format!("{form:?} × {taker:?}");
                let src = program(form, taker, n);
                match (n, verdict(&src)) {
                    (_, Verdict::Broke(why)) => {
                        off.push(format!("{label}, {n}: broke: {why}\n{src}"))
                    }
                    (1, Verdict::Ran) => {}
                    (0, Verdict::Refused(e)) if e.contains("can't prove `v > ") => {}
                    (_, got) => off.push(format!("{label}, {n}: {got:?}\n{src}")),
                }
            }
        }
    }
    assert!(off.is_empty(), "{} off:\n\n{}", off.len(), off.join("\n\n"));
}
