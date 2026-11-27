//! A generated family of programs that change a place in a value whose
//! refinement relates the places in it (docs/79-what-a-capture-splits.md):
//! each way something comes to be declared of a value (its origins: a
//! `let`'s or a parameter's refinement, what's behind a reference
//! parameter, a struct field's refinement, fields a refinement relates, a
//! tuple's element or a generic struct's field, nested too, of a `let`
//! without one, what a closure's parameter is given, what a `for`, a
//! `match` or a `let` taking a value apart binds, a copy of a variable
//! declared with one) × each way to change `P.x.n` (assigned, lent `&mut`
//! to a function or a closure, through a reference in a variable, a
//! pattern's `&mut` binding, a method taking `&mut self`, `swap`, and in a
//! closure by `&mut`, by a method, or `move`d), writing a value that
//! breaks `P`'s refinement (`x.n <= y`, `y` being 10: 50) or keeps it
//! whatever `y` is (0).
//!
//! The oracle is the interpreter's: no program accepted breaks a type at
//! run time; each that breaks the refinement is refused; each that changes
//! a value with nothing declared runs; and a write that keeps the
//! refinement is accepted where the checker sees the whole it's in (it's
//! assigned, or a closure captures the whole).
//!
//! They're about what programs mean, the same on every platform, so they
//! run on Linux alone.

use std::panic::catch_unwind;

/// Where what's declared of `P` comes from.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Origin {
    /// Nothing's declared: `let mut q = Q { .. }`.
    Plain,
    /// `let mut q: { v: Q | v.x.n <= v.y } = ..`.
    Annotated,
    /// `fn f(mut q: { v: Q | v.x.n <= v.y })`.
    Param,
    /// `fn f(r: &mut { v: Q | v.x.n <= v.y })`, `P` being `(*r)`.
    RefParam,
    /// `struct S { q: { v: Q | v.x.n <= v.y }, m: u32 }`, `P` being `s.q`.
    Field,
    /// `struct R { x: C, y: { h: u32 | h >= x.n } }`, `P` being `r`.
    Related,
    /// `let mut t = tuple();`, of type `({ v: Q | .. }, u32)`: `t.0`.
    Tuple,
    /// `let mut p = pair();`, a `Pair<{ v: Q | .. }>`: `p.a`.
    Generic,
    /// `let mut w = nested();`, a `Pair<W<{ v: Q | .. }>>`: `w.a.u`.
    Nested,
    /// `h(refined(), |mut t| ..)`: `t`.
    ClosureParam,
    /// `h(tuple(), |mut t| ..)`: `t.0`.
    ClosureParamTuple,
    /// `for mut t in l`, `l` a `List<({ v: Q | .. }, u32)>`: `t.0`.
    For,
    /// `match some() { Some(mut t) => .. }`: `t.0`.
    Match,
    /// `let (mut t, _) = apart();`: `t.0`.
    Destructure,
    /// `let a: ({ v: Q | .. }, u32) = ..; let mut t = a;`: `t.0`.
    Copy,
}

/// How `P.x.n` is changed.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Way {
    /// `P.x.n = V;`
    Assign,
    /// `set(&mut P.x, V);`
    Lend,
    /// `let r = &mut P.x; r.n = V;`
    RefLocal,
    /// `match &mut P.x { C { n } => *n = V }`
    Pattern,
    /// `P.x.set(V);`
    Method,
    /// `swap(&mut P.x, &mut o);`
    Swap,
    /// `let mut c = || P.x.n = V; c();`
    Closure,
    /// `let mut c = || P.x.set(V); c();`
    ClosureMethod,
    /// `let g = |c: &mut C| c.n = V; g(&mut P.x);`
    ClosureParam,
    /// `let mut c = move || { P.x.n = V; need(&P) }; c();`, `P` moved in.
    Move,
}

const ORIGINS: [Origin; 15] = [
    Origin::Plain,
    Origin::Annotated,
    Origin::Param,
    Origin::RefParam,
    Origin::Field,
    Origin::Related,
    Origin::Tuple,
    Origin::Generic,
    Origin::Nested,
    Origin::ClosureParam,
    Origin::ClosureParamTuple,
    Origin::For,
    Origin::Match,
    Origin::Destructure,
    Origin::Copy,
];
const WAYS: [Way; 10] = [
    Way::Assign,
    Way::Lend,
    Way::RefLocal,
    Way::Pattern,
    Way::Method,
    Way::Swap,
    Way::Closure,
    Way::ClosureMethod,
    Way::ClosureParam,
    Way::Move,
];

const COMMON: &str = "\
use alloc::list::List;
use core::mem::swap;

#[derive(Discard)]
struct C { n: u32 }

impl C {
    fn set(&mut self, v: u32) { self.n = v; }
}

#[derive(Discard)]
struct Q { x: C, y: u32 }

#[derive(Discard)]
struct S { q: { v: Q | v.x.n <= v.y }, m: u32 }

#[derive(Discard)]
struct R { x: C, y: { h: u32 | h >= x.n } }

#[derive(Discard)]
struct Pair<T> { a: T, n: u32 }

#[derive(Discard)]
struct W<U> { u: U, k: u32 }

fn set(c: &mut C, v: u32) { c.n = v; }

fn q() -> { v: Q | v.x.n <= v.y } { Q { x: C { n: 0 }, y: 10 } }

fn refined() -> { v: Q | v.x.n <= v.y } { q() }

fn tuple() -> ({ v: Q | v.x.n <= v.y }, u32) { (q(), 0) }

fn pair() -> Pair<{ v: Q | v.x.n <= v.y }> { Pair { a: q(), n: 0 } }

fn nested() -> Pair<W<{ v: Q | v.x.n <= v.y }>> { Pair { a: W { u: q(), k: 0 }, n: 0 } }

fn some() -> Option<({ v: Q | v.x.n <= v.y }, u32)> { Some((q(), 0)) }

fn apart() -> (({ v: Q | v.x.n <= v.y }, u32), u8) { ((q(), 0), 0) }

fn h<T, F: FnOnce(T)>(x: T, f: F) { f(x) }

fn need(q: &{ v: Q | v.x.n <= v.y }) with {} {
    let d: u32 = q.y - q.x.n;
    let _ = d;
}

fn need_r(r: &R) with {} {
    let d: u32 = r.y - r.x.n;
    let _ = d;
}

fn need_plain(q: &Q) with {} {
    let _ = q;
}

";

/// `P`, as written, and what needs its refinement, given `P` written.
fn place(origin: Origin) -> (&'static str, fn(&str) -> String) {
    let p = match origin {
        Origin::Plain | Origin::Annotated | Origin::Param => "q",
        Origin::RefParam => "(*r)",
        Origin::Field => "s.q",
        Origin::Related => "r",
        Origin::Generic => "p.a",
        Origin::Nested => "w.a.u",
        Origin::ClosureParam => "t",
        Origin::Tuple
        | Origin::ClosureParamTuple
        | Origin::For
        | Origin::Match
        | Origin::Destructure
        | Origin::Copy => "t.0",
    };
    let need: fn(&str) -> String = match origin {
        Origin::Plain => |p| format!("need_plain(&{p});"),
        Origin::Related => |p| format!("need_r(&{p});"),
        _ => |p| format!("need(&{p});"),
    };
    (p, need)
}

/// The statements changing `P.x.n` to `v`, then needing `P`'s refinement.
fn change(origin: Origin, way: Way, v: u32) -> String {
    let (p, need) = place(origin);
    let need = need(p);
    let change = match way {
        Way::Assign => format!("{p}.x.n = {v};"),
        Way::Lend => format!("set(&mut {p}.x, {v});"),
        Way::RefLocal => format!("let r2 = &mut {p}.x;\n        r2.n = {v};"),
        Way::Pattern => {
            format!("match &mut {p}.x {{\n            C {{ n }} => *n = {v},\n        }}")
        }
        Way::Method => format!("{p}.x.set({v});"),
        Way::Swap => format!("let mut o = C {{ n: {v} }};\n        swap(&mut {p}.x, &mut o);"),
        Way::Closure => format!("let mut c = || {p}.x.n = {v};\n        c();"),
        Way::ClosureMethod => format!("let mut c = || {p}.x.set({v});\n        c();"),
        Way::ClosureParam => {
            format!("let g = |c: &mut C| c.n = {v};\n        g(&mut {p}.x);")
        }
        // `P` is moved in: what needs it is in the closure.
        Way::Move => {
            return format!(
                "let mut c = move || {{\n            {p}.x.n = {v};\n            {need}\n        }};\n        c();"
            );
        }
    };
    format!("{change}\n        {need}")
}

fn program(origin: Origin, way: Way, v: u32) -> String {
    let body = change(origin, way, v);
    let main = "#[allow(unused_mut, unused_variables)]\nfn main() with {alloc, panic} {\n";
    let code = match origin {
        Origin::Plain => {
            format!("{main}    let mut q = q();\n    {{\n        {body}\n    }}\n}}\n")
        }
        Origin::Annotated => format!(
            "{main}    let mut q: {{ v: Q | v.x.n <= v.y }} = q();\n    {{\n        {body}\n    }}\n}}\n"
        ),
        Origin::Param => format!(
            "#[allow(unused_mut, unused_variables)]\nfn f(mut q: {{ v: Q | v.x.n <= v.y }}) with {{alloc, panic}} {{\n    \
             {{\n        {body}\n    }}\n}}\n\n{main}    f(refined());\n}}\n"
        ),
        Origin::RefParam => format!(
            "#[allow(unused_mut, unused_variables)]\nfn f(r: &mut {{ v: Q | v.x.n <= v.y }}) with {{alloc, panic}} {{\n    \
             {{\n        {body}\n    }}\n}}\n\n{main}    let mut q: {{ v: Q | v.x.n <= v.y }} = q();\n    f(&mut q);\n}}\n"
        ),
        Origin::Field => format!(
            "{main}    let mut s = S {{ q: q(), m: 0 }};\n    {{\n        {body}\n    }}\n}}\n"
        ),
        Origin::Related => format!(
            "{main}    let mut r = R {{ x: C {{ n: 0 }}, y: 10 }};\n    {{\n        {body}\n    }}\n}}\n"
        ),
        Origin::Tuple => {
            format!("{main}    let mut t = tuple();\n    {{\n        {body}\n    }}\n}}\n")
        }
        Origin::Generic => {
            format!("{main}    let mut p = pair();\n    {{\n        {body}\n    }}\n}}\n")
        }
        Origin::Nested => {
            format!("{main}    let mut w = nested();\n    {{\n        {body}\n    }}\n}}\n")
        }
        Origin::ClosureParam => {
            format!("{main}    h(refined(), |mut t| {{\n        {body}\n    }});\n}}\n")
        }
        Origin::ClosureParamTuple => {
            format!("{main}    h(tuple(), |mut t| {{\n        {body}\n    }});\n}}\n")
        }
        Origin::For => format!(
            "{main}    let mut l: List<({{ v: Q | v.x.n <= v.y }}, u32)> = List::new();\n    \
             l.push((q(), 0));\n    for mut t in l {{\n        {body}\n    }}\n}}\n"
        ),
        Origin::Match => format!(
            "{main}    match some() {{\n        Some(mut t) => {{\n        {body}\n        }},\n        \
             None => {{}},\n    }}\n}}\n"
        ),
        Origin::Destructure => {
            format!("{main}    let (mut t, _) = apart();\n    {{\n        {body}\n    }}\n}}\n")
        }
        Origin::Copy => format!(
            "{main}    let a: ({{ v: Q | v.x.n <= v.y }}, u32) = (q(), 0);\n    let mut t = a;\n    \
             {{\n        {body}\n    }}\n}}\n"
        ),
    };
    format!("{COMMON}{code}")
}

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

/// What's expected of a program: `Some(true)` accepted, `Some(false)`
/// refused, `None` either (but never breaking a type at run time).
fn expected(origin: Origin, way: Way, breaks: bool) -> Option<bool> {
    match origin {
        // Nothing's declared: every way runs.
        Origin::Plain => Some(true),
        // Every refinement broken is refused, however it's written.
        _ if breaks => Some(false),
        // What a closure's parameter is declared is which values the call
        // has of its type, which a change may not keep.
        Origin::ClosureParam | Origin::ClosureParamTuple => None,
        // A write the checker sees the whole of keeps it: assigned, or
        // through a closure that captures the whole.
        _ if matches!(way, Way::Assign | Way::Closure | Way::Move) => Some(true),
        _ => None,
    }
}

/// Each program breaking `P`'s refinement is refused, whatever declared
/// it and however it's changed; none accepted breaks a type at run time.
#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn every_origin_every_way() {
    let mut off = Vec::new();
    for origin in ORIGINS {
        for way in WAYS {
            for (breaks, v) in [(true, 50), (false, 0)] {
                let src = program(origin, way, v);
                let label = format!(
                    "{origin:?} × {way:?} × {}",
                    if breaks { "breaks" } else { "keeps" }
                );
                match (expected(origin, way, breaks), verdict(&src)) {
                    (_, Verdict::Broke(why)) => off.push(format!("{label}: broke: {why}\n{src}")),
                    (Some(true) | None, Verdict::Ran)
                    | (Some(false) | None, Verdict::Refused(_)) => {}
                    (_, Verdict::Ran) => off.push(format!("{label}: accepted\n{src}")),
                    (_, Verdict::Refused(e)) => off.push(format!("{label}: refused: {e}\n{src}")),
                }
            }
        }
    }
    assert!(off.is_empty(), "{} off:\n\n{}", off.len(), off.join("\n\n"));
}
