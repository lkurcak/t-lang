//! A generated family of programs that give a bounded type parameter a
//! type argument: each way a bound can say something of a type parameter,
//! at each position a bound can be in (docs/70-bounds-in-impls.md). Each
//! is the product of
//!
//! - a **kind** of bound: a trait (`T: Copy`), an equality (`I:
//!   Iterator<Item = u8>`), an associated effect (`I: Iterator<Item = u8,
//!   e = {}>`), or a closure's effects (`F: FnMut() -> u8 with {}`);
//! - a **position**: a function's own bound; a trait's method's, the same
//!   in its impl; an impl's method's, stronger than its trait's, or
//!   weaker; an impl's `where`; an opaque result's; a function made a
//!   function pointer, or called in a closure;
//! - an **argument**: one the bound doesn't hold of, which would break
//!   what the body counts on (a `List` copied, a `char` read as a `u8`, a
//!   panic in a function that says `with {}`), or one it holds of.
//!
//! The oracle is the interpreter's: every program is refused, or runs
//! without "type soundness violated". Each program whose argument breaks
//! the bound is refused, and each whose argument meets it runs, where the
//! position lets it be written.
//!
//! Retrospective 8 found an impl's method could say more than its trait's
//! of an equality or an associated effect: the calls through the trait
//! never showed it held.
//!
//! They're about what programs mean, the same on every platform, so they run
//! on Linux alone.

use std::panic::catch_unwind;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    /// `T: Copy`: the body copies `x`. Broken by a `List`, freed twice.
    Trait,
    /// `I: Iterator<Item = u8>`: the body gives an item as a `u8`. Broken
    /// by `Chars`, whose items are `char`s.
    Equality,
    /// `I: Iterator<Item = u8, e = {}>`: the body says `with {}`. Broken by
    /// `Boom`, whose `next` panics.
    Effect,
    /// `F: FnMut() -> u8 with {}`: the body says `with {}`. Broken by
    /// `boom`, which panics.
    Closure,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Position {
    /// `fn take<B>(x: P) -> u8 { .. }`, called.
    Fn,
    /// A trait's method and its impl's, both bounded, called through the
    /// trait.
    Trait,
    /// An impl's method bounded, its trait's not: a call through the trait
    /// is checked against the trait's.
    ImplStronger,
    /// A trait's method bounded, its impl's not.
    ImplWeaker,
    /// `impl<P> Get for W<P> where B`, used through `J: Get`.
    ImplWhere,
    /// `fn mk() -> impl B { arg }`, given to `take`.
    Opaque,
    /// `let f: fn(A) -> u8 = take; f(arg)`.
    FnPointer,
    /// `let c = |a: A| take(a); c(arg)`.
    Closure,
}

const KINDS: [Kind; 4] = [Kind::Trait, Kind::Equality, Kind::Effect, Kind::Closure];
const POSITIONS: [Position; 8] = [
    Position::Fn,
    Position::Trait,
    Position::ImplStronger,
    Position::ImplWeaker,
    Position::ImplWhere,
    Position::Opaque,
    Position::FnPointer,
    Position::Closure,
];

/// What every program has: an iterator whose `next` panics, and functions
/// that panic or don't, to be closures.
const ITEMS: &str = "use alloc::list::List;
use core::ops::Range;
use core::str::Chars;

#[derive(Discard)]
struct Boom {
    n: u8,
}

impl Iterator for Boom {
    type Item = u8;
    effect e = panic;

    fn next(&mut self) -> Option<u8> with panic { panic(\"boom\") }
}

fn boom() -> u8 with panic { panic(\"boom\") }

fn one() -> u8 { 1 }

";

impl Kind {
    /// The type parameter the bound is on.
    fn param(self) -> &'static str {
        match self {
            Kind::Trait => "T",
            Kind::Equality | Kind::Effect => "I",
            Kind::Closure => "F",
        }
    }

    /// The parameters, without the bound: an effect parameter for a
    /// closure's.
    fn params(self) -> &'static str {
        match self {
            Kind::Closure => "effect g, F",
            k => k.param(),
        }
    }

    /// The bound on the parameter, or what's left of it weakened.
    fn bound(self, strong: bool) -> &'static str {
        match (self, strong) {
            (Kind::Trait, true) => "T: Copy + Discard",
            (Kind::Trait, false) => "T: Discard",
            (Kind::Equality, true) => "I: Iterator<Item = u8> + Discard",
            (Kind::Equality, false) => "I: Iterator + Discard",
            (Kind::Effect, true) => "I: Iterator<Item = u8, e = {}> + Discard",
            (Kind::Effect, false) => "I: Iterator<Item = u8> + Discard",
            (Kind::Closure, true) => "F: FnMut() -> u8 with {} + Discard",
            (Kind::Closure, false) => "F: FnMut() -> u8 with g + Discard",
        }
    }

    /// The generics of a function bounded so.
    fn generics(self, strong: bool) -> String {
        match self {
            Kind::Closure => format!("effect g, {}", self.bound(strong)),
            k => k.bound(strong).to_string(),
        }
    }

    /// What the opaque type is: iterable in `go`, which says its effects.
    fn opaque(self) -> &'static str {
        match self {
            Kind::Trait => "Copy + Discard",
            Kind::Equality | Kind::Effect => "Iterator<Item = u8, e = {}> + Discard",
            Kind::Closure => "FnMut() -> u8 with {} + Discard",
        }
    }

    /// What the function's body may do.
    fn effects(self, strong: bool) -> &'static str {
        match (self, strong) {
            (Kind::Trait, _) => "{}",
            (Kind::Equality, _) => "I::e",
            (Kind::Effect, true) => "{}",
            (Kind::Effect, false) => "I::e",
            (Kind::Closure, true) => "{}",
            (Kind::Closure, false) => "g",
        }
    }

    /// The body, which counts on the bound, if `strong`, of `mut x: P`.
    fn body(self, strong: bool) -> &'static str {
        match (self, strong) {
            (Kind::Trait, true) => "let a = x;\n        let b = x;\n        1",
            (Kind::Trait, false) => "1",
            (Kind::Equality, true) | (Kind::Effect, _) => {
                "match x.next() {\n            Some(v) => v,\n            None => 0,\n        }"
            }
            (Kind::Equality, false) => "0",
            (Kind::Closure, _) => "x()",
        }
    }

    /// An argument the bound doesn't hold of, or one it does.
    fn arg(self, breaks: bool) -> &'static str {
        match (self, breaks) {
            (Kind::Trait, true) => "List::filled(1, 1u8)",
            (Kind::Trait, false) => "5u8",
            (Kind::Equality, true) => "\"ab\".chars()",
            (Kind::Equality | Kind::Effect, false) => "1u8..3",
            (Kind::Effect, true) => "Boom { n: 0 }",
            (Kind::Closure, true) => "boom",
            (Kind::Closure, false) => "one",
        }
    }

    /// The argument's type.
    fn arg_ty(self, breaks: bool) -> &'static str {
        match (self, breaks) {
            (Kind::Trait, true) => "List<u8>",
            (Kind::Trait, false) => "u8",
            (Kind::Equality, true) => "Chars",
            (Kind::Equality | Kind::Effect, false) => "Range<u8>",
            (Kind::Effect, true) => "Boom",
            (Kind::Closure, true) => "fn() -> u8 with panic",
            (Kind::Closure, false) => "fn() -> u8",
        }
    }
}

/// `take`, bounded so.
fn take(k: Kind) -> String {
    format!(
        "fn take<{}>(mut x: {}) -> u8 with {} {{\n    {}\n}}\n",
        k.generics(true),
        k.param(),
        k.effects(true),
        k.body(true)
            .replace("\n        ", "\n    ")
            .replace("\n    }", "\n}")
    )
}

/// A trait `Run` whose method is bounded `trait_strong`, implemented by
/// `R` with one bounded `impl_strong`.
fn run(k: Kind, trait_strong: bool, impl_strong: bool) -> String {
    format!(
        "trait Run {{\n    fn run<{}>(x: {}) -> u8 with {};\n}}\n\n\
         #[derive(Discard)]\nstruct R {{}}\n\n\
         impl Run for R {{\n    fn run<{}>(mut x: {}) -> u8 with {} {{\n        {}\n    }}\n}}\n\n\
         fn via<X: Run>() -> u8 with {{alloc, panic}} {{ X::run({{arg}}) }}\n\n\
         fn go() -> u8 with {{alloc, panic}} {{ via::<R>() }}\n",
        k.generics(trait_strong),
        k.param(),
        k.effects(trait_strong),
        k.generics(impl_strong),
        k.param(),
        k.effects(impl_strong),
        k.body(impl_strong),
    )
}

fn program(k: Kind, p: Position, breaks: bool) -> String {
    let arg = k.arg(breaks);
    let ty = k.arg_ty(breaks);
    let body = match p {
        Position::Fn => format!(
            "{}\nfn go() -> u8 with {{alloc, panic}} {{ take({arg}) }}\n",
            take(k)
        ),
        Position::Trait => run(k, true, true),
        Position::ImplStronger => run(k, false, true),
        Position::ImplWeaker => run(k, true, false),
        Position::ImplWhere => format!(
            "#[derive(Discard)]\nstruct W<{0}> {{\n    x: {0},\n}}\n\n\
             trait Get {{\n    effect e;\n\n    fn get(self) -> u8 with Self::e;\n}}\n\n\
             impl<{1}> Get for W<{0}>\nwhere\n    {2},\n{{\n    effect e = {3};\n\n    \
             fn get(self) -> u8 with {3} {{\n        let mut x = self.x;\n        {4}\n    }}\n}}\n\n\
             fn via<J: Get>(j: J) -> u8 with J::e {{ j.get() }}\n\n\
             fn go() -> u8 with {{alloc, panic}} {{ via(W {{ x: {arg} }}) }}\n",
            k.param(),
            k.params(),
            k.bound(true),
            k.effects(true),
            k.body(true),
        ),
        Position::Opaque => format!(
            "{}\nfn mk() -> impl {} with alloc {{ {arg} }}\n\n\
             fn go() -> u8 with {{alloc, panic}} {{ take(mk()) }}\n",
            take(k),
            k.opaque(),
        ),
        Position::FnPointer => format!(
            "{}\nfn go() -> u8 with {{alloc, panic}} {{\n    let f: fn({ty}) -> u8 = take;\n    f({arg})\n}}\n",
            take(k)
        ),
        Position::Closure => format!(
            "{}\nfn go() -> u8 with {{alloc, panic}} {{\n    let c = |a: {ty}| take(a);\n    c({arg})\n}}\n",
            take(k)
        ),
    };
    format!(
        "{ITEMS}{}\nfn main(os: std::Os) -> Result<(), std::io::Error> with {{alloc, panic}} {{\n    \
         let mut out = os.stdout;\n    out.line(&go())?;\n    Ok(())\n}}\n",
        body.replace("{arg}", arg)
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

fn all() -> Vec<(String, bool, String)> {
    let mut out = Vec::new();
    for k in KINDS {
        for p in POSITIONS {
            for breaks in [true, false] {
                let label = format!("{k:?} × {p:?}");
                out.push((label, breaks, program(k, p, breaks)));
            }
        }
    }
    out
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn no_bound_is_broken() {
    let mut broke = Vec::new();
    for (label, breaks, src) in all() {
        if let Verdict::Broke(why) = verdict(&src) {
            let arg = match breaks {
                true => "breaking it",
                false => "meeting it",
            };
            broke.push(format!("{label}, an argument {arg}: {why}\n{src}"));
        }
    }
    assert!(broke.is_empty(), "{}", broke.join("\n\n"));
}

/// Every program whose argument breaks the bound is refused: if one ran,
/// it would break it, which the oracle sees, so this says the family's
/// arguments do break their bounds, and that the run above means
/// something.
#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn each_breaking_argument_is_refused() {
    let mut off = Vec::new();
    for (label, breaks, src) in all() {
        if breaks && !matches!(verdict(&src), Verdict::Refused(_)) {
            off.push(format!("{label}\n{src}"));
        }
    }
    assert!(off.is_empty(), "{}", off.join("\n\n"));
}

/// Each program whose argument meets the bound runs, where the position
/// lets it be written: an impl's method bounded differently from its
/// trait's is refused whatever it's given (lemma I1 of
/// docs/70-bounds-in-impls.md), and an impl can't bound an associated
/// effect (docs/68.1-effect-bindings.md).
#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn each_fitting_argument_runs() {
    let mut off = Vec::new();
    for k in KINDS {
        for p in POSITIONS {
            let refused = match p {
                Position::ImplStronger | Position::ImplWeaker => true,
                Position::ImplWhere => matches!(k, Kind::Effect | Kind::Closure),
                _ => false,
            };
            let src = program(k, p, false);
            let got = verdict(&src);
            let ok = match refused {
                true => matches!(got, Verdict::Refused(_)),
                false => got == Verdict::Ran,
            };
            if !ok {
                off.push(format!("{k:?} × {p:?}: {got:?}\n{src}"));
            }
        }
    }
    assert!(off.is_empty(), "{}", off.join("\n\n"));
}

/// An impl's method bounded differently from its trait's is refused for
/// that, not for something else in the program.
#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn an_impl_method_bounded_otherwise_is_refused_for_it() {
    let mut off = Vec::new();
    for k in KINDS {
        for p in [Position::ImplStronger, Position::ImplWeaker] {
            for breaks in [true, false] {
                let src = program(k, p, breaks);
                match verdict(&src) {
                    Verdict::Refused(e) if e.contains("doesn't match its declaration") => {}
                    // A weaker impl's effects are more than the trait's,
                    // which is refused as before.
                    Verdict::Refused(e)
                        if p == Position::ImplWeaker && e.contains("so its impls can't either") => {
                    }
                    got => off.push(format!("{k:?} × {p:?}, breaking {breaks}: {got:?}\n{src}")),
                }
            }
        }
    }
    assert!(off.is_empty(), "{}", off.join("\n\n"));
}
