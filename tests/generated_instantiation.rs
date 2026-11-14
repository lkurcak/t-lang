//! A generated family of programs that give a generic function something
//! refined and get it back (docs/74-refined-instantiation.md):
//!
//! - **lends**: a refined place, of each kind (a local, a field about it
//!   alone, a field naming another, an enum's type argument, behind a
//!   refined reference), lent `&mut` to a generic function of each kind
//!   (`swap` with another place, `replace`, a `set` of our own, a closure
//!   given the `&mut`, one storing what a closure gives), which writes a
//!   value that keeps the refinement or breaks it;
//! - **gives**: a closure written in a call, of each body (a literal, a
//!   call, a branch, a capture), its value in each shape of the result (`R`,
//!   `Option<R>`, a tuple's element, run twice), given to each kind of
//!   generic function (`call`, `Mutex::lock`, `unwrap_or_else`,
//!   `Option::map`, a thread's `join`), whose result is a refined `let`.
//!
//! The oracle is the interpreter's: every program is refused, or runs
//! without "type soundness violated". Each program writing a value that
//! keeps the refinement is accepted, and each writing one that breaks it is
//! refused for the refinement.
//!
//! They're about what programs mean, the same on every platform, so they run
//! on Linux alone.

use std::panic::catch_unwind;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Place {
    /// `let mut a: { v: u32 | v > 0 } = 1;`, lent `&mut a`.
    Local,
    /// `p.n` of `struct P { n: { v: u32 | v > 0 } }`.
    Alone,
    /// `q.n` of `struct Q { k: u32, n: { v: u32 | v > k } }`, `k` being 0.
    Sibling,
    /// `o`, an `Option<{ v: u32 | v > 0 }>`, its value `Some(..)`.
    Part,
    /// `*r`, of a parameter `r: &mut { v: u32 | v > 0 }`.
    Behind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Lend {
    /// `swap(&mut place, &mut other)`, `other` holding the value, declared
    /// as the place is if the value keeps it, else not refined.
    Swap,
    /// `let _ = replace(&mut place, value);`
    Replace,
    /// `set(&mut place, value);`
    Set,
    /// `with_mut(&mut place, |x| *x = value);`
    Closure,
    /// `store(&mut place, || value);`
    Store,
}

const PLACES: [Place; 5] = [
    Place::Local,
    Place::Alone,
    Place::Sibling,
    Place::Part,
    Place::Behind,
];
const LENDS: [Lend; 5] = [
    Lend::Swap,
    Lend::Replace,
    Lend::Set,
    Lend::Closure,
    Lend::Store,
];

const LEND_ITEMS: &str = "\
use core::mem::{replace, swap};

fn set<T: Discard>(a: &mut T, v: T) {
    *a = v;
}

fn with_mut<T, F: FnOnce(&mut T) + Discard>(a: &mut T, f: F) { f(a) }

fn store<R: Discard, F: FnOnce() -> R + Discard>(dst: &mut R, f: F) { *dst = f(); }

#[derive(Copy, Discard)]
struct P { n: { v: u32 | v > 0 } }

#[derive(Copy, Discard)]
struct Q { k: u32, n: { v: u32 | v > k } }

";

impl Place {
    /// The type of a value written there, and the refinement of `other`
    /// that `swap` needs: the place's own, as a type.
    fn ty(self) -> (&'static str, &'static str) {
        match self {
            Place::Part => ("Option<u32>", "Option<{ v: u32 | v > 0 }>"),
            _ => ("u32", "{ v: u32 | v > 0 }"),
        }
    }

    /// The value written, `n` (0 breaks the refinement, 1 keeps it).
    fn value(self, n: u32) -> String {
        match self {
            Place::Part => format!("Some({n})"),
            _ => n.to_string(),
        }
    }
}

/// `fn f`, which lends `place` the way `lend` does and writes `n`, and the
/// `main` that calls it and reads the place after.
fn lend_program(place: Place, lend: Lend, n: u32) -> String {
    let (ty, refined) = place.ty();
    let value = place.value(n);
    let (setup, at, read, param) = match place {
        Place::Local => (
            "let mut a: { v: u32 | v > 0 } = 1;".to_string(),
            "a",
            "a",
            "",
        ),
        Place::Alone => ("let mut p = P { n: 1 };".to_string(), "p.n", "p.n", ""),
        Place::Sibling => (
            "let mut q = Q { k: 0, n: 1 };".to_string(),
            "q.n",
            "q.n",
            "",
        ),
        Place::Part => (
            "let mut o: Option<{ v: u32 | v > 0 }> = Some(1);".to_string(),
            "o",
            "o.unwrap_or(1)",
            "",
        ),
        Place::Behind => (String::new(), "*r", "*r", "r: &mut { v: u32 | v > 0 }"),
    };
    let other_ty = if n == 0 { ty } else { refined };
    let code = match lend {
        Lend::Swap => {
            format!("let mut other: {other_ty} = {value};\n    swap(&mut {at}, &mut other);")
        }
        Lend::Replace => format!("let _ = replace(&mut {at}, {value});"),
        Lend::Set => format!("set(&mut {at}, {value});"),
        Lend::Closure => format!("with_mut(&mut {at}, |x| *x = {value});"),
        Lend::Store => format!("store(&mut {at}, || {value});"),
    };
    let f = format!(
        "fn f({param}) -> u32 with panic {{\n    {setup}\n    {code}\n    let got: {{ v: u32 | v > 0 }} = {read};\n    got\n}}\n"
    );
    let call = match place {
        Place::Behind => "let mut b: { v: u32 | v > 0 } = 1;\n    let _x = f(&mut b);",
        _ => "let _x = f();",
    };
    format!("{LEND_ITEMS}{f}\nfn main() with panic {{\n    {call}\n}}\n")
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Body {
    /// `|| v`
    Literal,
    /// `|| keep(v)`, a call whose result is what it's given.
    Call,
    /// `|| { if c { return v; } 1 }`
    Branch,
    /// `move || k`, of a `k` that's `v`.
    Capture,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Shape {
    /// The closure gives `R` itself.
    Whole,
    /// It gives `Some(..)`, an `Option<R>`, unwrapped after.
    Option,
    /// It gives `(.., 7)`, of which the call gives the first.
    Tuple,
    /// It runs twice, each result read.
    Twice,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Taker {
    /// `call(f)`, of `fn call<R, F: FnOnce() -> R>`.
    Call,
    /// `m.lock(|_| ..)`, of a `Mutex`.
    Lock,
    /// `none.unwrap_or_else(f)`.
    OrElse,
    /// `Some(9).map(|_| ..)`.
    Map,
    /// `os.threads.spawn(f).join()`.
    Join,
}

const BODIES: [Body; 4] = [Body::Literal, Body::Call, Body::Branch, Body::Capture];
const SHAPES: [Shape; 4] = [Shape::Whole, Shape::Option, Shape::Tuple, Shape::Twice];
const TAKERS: [Taker; 5] = [
    Taker::Call,
    Taker::Lock,
    Taker::OrElse,
    Taker::Map,
    Taker::Join,
];

const GIVE_ITEMS: &str = "\
use core::sync::Mutex;

fn call<R, effect e, F: FnOnce() -> R with e + Discard>(f: F) -> R with e { f() }

fn twice<R, F: FnMut() -> R + Discard>(mut f: F) -> (R, R) { (f(), f()) }

fn keep(x: u32) -> { v: u32 | v == x } { x }

fn first<A, B: Discard>(p: (A, B)) -> A {
    let (a, _) = p;
    a
}

";

/// A program whose closure gives `n` (0 breaks `v > 0`, 1 keeps it) in
/// `shape`, from `body`, to `taker`, read into a refined `let`.
fn give_program(body: Body, shape: Shape, taker: Taker, n: u32) -> String {
    let inner = match body {
        Body::Literal => n.to_string(),
        Body::Call => format!("keep({n})"),
        Body::Branch => {
            format!("{{\n        if c {{\n            return {n};\n        }}\n        1\n    }}")
        }
        Body::Capture => "k".to_string(),
    };
    // What the closure's body gives, in the shape.
    let made = match shape {
        Shape::Whole | Shape::Twice => inner.clone(),
        Shape::Option => match body {
            Body::Branch => format!(
                "{{\n        if c {{\n            return Some({n});\n        }}\n        Some(1)\n    }}"
            ),
            _ => format!("Some({inner})"),
        },
        Shape::Tuple => match body {
            Body::Branch => format!(
                "{{\n        if c {{\n            return ({n}, 7u8);\n        }}\n        (1, 7u8)\n    }}"
            ),
            _ => format!("({inner}, 7u8)"),
        },
    };
    let mv = if body == Body::Capture { "move " } else { "" };
    let (closure, unit_closure) = (format!("{mv}|| {made}"), format!("{mv}|_| {made}"));
    // The call, giving a value of the closure's result's type.
    let got = match (taker, shape) {
        (_, Shape::Twice) => format!("twice({closure})"),
        (Taker::Call, _) => format!("call({closure})"),
        (Taker::Lock, _) => format!("m.lock({unit_closure})"),
        (Taker::OrElse, _) => {
            let ty = match shape {
                Shape::Option => "Option<Option<u32>>",
                Shape::Tuple => "Option<(u32, u8)>",
                _ => "Option<u32>",
            };
            format!(
                "{{\n        let none: {ty} = None;\n        none.unwrap_or_else({closure})\n    }}"
            )
        }
        (Taker::Map, _) => format!("Some(9u8).map({unit_closure}).unwrap()"),
        (Taker::Join, _) => format!("os.threads.spawn({closure}).join()"),
    };
    let read = match shape {
        Shape::Whole => format!("let r: {{ v: u32 | v > 0 }} = {got};"),
        Shape::Option => format!("let r: {{ v: u32 | v > 0 }} = {got}.unwrap();"),
        Shape::Tuple => format!("let r: {{ v: u32 | v > 0 }} = first({got});"),
        Shape::Twice => format!(
            "let (a, b) = {got};\n    let r: {{ v: u32 | v > 0 }} = a;\n    let s: {{ v: u32 | v > 0 }} = b;\n    let _ = s;"
        ),
    };
    format!(
        "{GIVE_ITEMS}fn main(os: std::Os) with {{alloc, panic}} {{\n    let c = true;\n    let k: u32 = {n};\n    let m: Mutex<u8> = Mutex::new(0);\n    {read}\n    let _ = (c, k, m, r, os);\n}}\n"
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

fn all() -> Vec<(String, u32, String)> {
    let mut out = Vec::new();
    for n in [0, 1] {
        for place in PLACES {
            for lend in LENDS {
                let label = format!("{place:?} × {lend:?}");
                out.push((label, n, lend_program(place, lend, n)));
            }
        }
        for body in BODIES {
            for shape in SHAPES {
                for taker in TAKERS {
                    // `twice` is its own taker.
                    if shape == Shape::Twice && taker != Taker::Call {
                        continue;
                    }
                    let label = format!("{body:?} × {shape:?} × {taker:?}");
                    out.push((label, n, give_program(body, shape, taker, n)));
                }
            }
        }
    }
    out
}

/// Each program keeping the refinement is accepted and runs; each breaking
/// it is refused, for the refinement; none breaks it at run time.
#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn what_comes_back_has_what_went_in() {
    let mut off = Vec::new();
    for (label, n, src) in all() {
        match (n, verdict(&src)) {
            (_, Verdict::Broke(why)) => off.push(format!("{label}, {n}: broke: {why}\n{src}")),
            (1, Verdict::Ran) => {}
            (0, Verdict::Refused(e)) if e.contains("can't prove `v > ") => {}
            (_, got) => off.push(format!("{label}, {n}: {got:?}\n{src}")),
        }
    }
    assert!(off.is_empty(), "{} off:\n\n{}", off.len(), off.join("\n\n"));
}
