//! Deterministic families for an impl's associated effect with an atom
//! that hangs on a bound, `effect e = panic unless T: Low`
//! (docs/96-associated-effects-unless.md). In code generic in a `T` that
//! may or may not be `Low`, `<W<T> as Go>::e` stays itself: it's at most
//! `panic` in a set within another, and never taken to be `panic` in the
//! bigger one. So the first family puts it in each place a set must be
//! within it, and gives each something that keeps that or breaks it; the
//! second varies the impl's own type, how the bound's impl is given of
//! the type it's called at, and how the impl's method says its effects.
//! Each program is refused, or accepted and run by the interpreter, which
//! checks at every panic that each function being run says it, in the
//! copy that's run. Every one that breaks what it says panics in the copy
//! that's `Low`, so one let through is caught there. They're about what
//! programs mean, so they run on Linux alone.

use t_lang::error::Phase;

/// Checks `src` and, if it's accepted, runs it. Gives the error's message
/// if it's refused, or what went wrong running it.
fn verdict(src: &str) -> Result<(), String> {
    let checked = t_lang::check_source(src);
    let program = match checked.result {
        Ok(p) => p,
        Err(e) => {
            assert_eq!(
                e.phase,
                Phase::Type,
                "{src}\n{}",
                e.render_in(&checked.sources)
            );
            return Err(e.message);
        }
    };
    let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut terminal = t_lang::interp::Captured::default();
        t_lang::interp::run(&program.lower(), &mut terminal)
    }));
    match run {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(format!(
            "accepted, but it failed: {}",
            e.render_in(&checked.sources)
        )),
        Err(_) => Err("accepted, but the interpreter found an effect not said".to_string()),
    }
}

/// `Lo` is `Low`, and its value the greatest the law allows, so anything
/// more than one added to it overflows; `Hi` isn't. `W<T>`'s `go` may
/// panic only where `T` isn't `Low`. `boom` panics; `Down` does when
/// iterated, and `Up` never.
const PRELUDE: &str = r#"trait Val {
    fn value(&self) -> u8;
}

trait Low: Val {
    law fn value(&self) -> { r: u8 | r < 255 };
}

#[derive(Discard)]
struct Lo { n: u8 }

#[derive(Discard)]
struct Hi { n: u8 }

impl Val for Lo {
    fn value(&self) -> u8 { 254 }
}

impl Low for Lo {}

impl Val for Hi {
    fn value(&self) -> u8 { 3 }
}

trait Go {
    effect e;

    fn go(&self) -> u8 with Self::e;
}

#[derive(Discard where T: Discard)]
struct W<T> { x: T }

impl<T: Val> Go for W<T> {
    effect e = panic unless T: Low;

    fn go(&self) -> u8 with panic unless T: Low { self.x.value() + 1 }
}

fn boom() -> u8 with panic { panic("boom") }

fn fine() -> u8 { 1 }

#[derive(Discard)]
struct Down { n: u8 }

impl Iterator for Down {
    type Item = u8;
    effect e = panic;

    fn next(&mut self) -> Option<u8> with panic { panic("down") }
}

#[derive(Discard)]
struct Up { n: u8 }

impl Iterator for Up {
    type Item = u8;
    effect e = {};

    fn next(&mut self) -> Option<u8> { None }
}
"#;

/// A place `X`, `<W<T> as Go>::e`, is the bigger set, with what's given
/// there keeping it (`honest`) or not (`lie`), and `use_` calling it, of
/// `w`, a `&W<T>`.
struct Position {
    name: &'static str,
    items: &'static str,
    honest: &'static str,
    lie: &'static str,
    use_: &'static str,
}

const POSITIONS: [Position; 7] = [
    Position {
        name: "what a function says",
        items: "fn h<T: Val>(w: &W<T>) -> u8 with X { BODY }",
        honest: "w.x.value() + 1",
        lie: "w.x.value() + 2",
        use_: "h(&w)",
    },
    Position {
        name: "what an inherent method says",
        items: "#[derive(Discard where T: Discard)]\nstruct H<T> { w: W<T> }\n\n\
                impl<T: Val> H<T> {\n    fn h(&self) -> u8 with <W<T> as Go>::e { BODY }\n}",
        honest: "self.w.x.value() + 1",
        lie: "self.w.x.value() + 2",
        use_: "(H { w: W { x: w.x } }).h()",
    },
    Position {
        name: "a function value",
        items: "fn h<T: Val>(_w: &W<T>) -> u8 with X {\n    let f: fn() -> u8 with X = BODY;\n    f()\n}",
        honest: "fine",
        lie: "boom",
        use_: "h(&w)",
    },
    Position {
        name: "an `Fn` bound",
        items: "fn apply<T: Val, F: Fn() -> u8 with X + Discard>(_w: &W<T>, f: F) -> u8 with X { f() }\n\n\
                fn h<T: Val>(w: &W<T>) -> u8 with X { apply(w, BODY) }",
        honest: "|| 1u8",
        lie: "|| boom()",
        use_: "h(&w)",
    },
    Position {
        name: "a bound on an iterator's effect",
        items: "fn first<T: Val, J: Iterator<Item = u8, e = X> + Discard>(_w: &W<T>, mut j: J) -> u8 with X {\n    \
                let _r = j.next();\n    j.discard();\n    1\n}\n\n\
                fn h<T: Val>(w: &W<T>) -> u8 with X { first(w, BODY) }",
        honest: "Up { n: 0 }",
        lie: "Down { n: 0 }",
        use_: "h(&w)",
    },
    Position {
        name: "an opaque result's",
        items: "fn mk<T: Val>(_w: &W<T>) -> impl Iterator<Item = u8, e = X> + Discard { BODY }\n\n\
                fn h<T: Val>(w: &W<T>) -> u8 with X {\n    let mut it = mk(w);\n    \
                let _r = it.next();\n    it.discard();\n    1\n}",
        honest: "Up { n: 0 }",
        lie: "Down { n: 0 }",
        use_: "h(&w)",
    },
    Position {
        name: "an effect parameter",
        items: "fn pick<effect fx>(a: fn() -> u8 with fx, _b: fn() -> u8 with fx) -> fn() -> u8 with fx { a }\n\n\
                fn h<T: Val>(_w: &W<T>) -> u8 with X {\n    let f: fn() -> u8 with X = pick(BODY, fine);\n    f()\n}",
        honest: "fine",
        lie: "boom",
        use_: "h(&w)",
    },
];

/// A program with position `p` given `body`, and a `main` calling it of
/// `W<Lo>`, saying nothing, if `low`, or of `W<Hi>`, saying `panic`.
fn positioned(p: &Position, body: &str, low: bool) -> String {
    let items = (p.items.replace("BODY", body)).replace("with X", "with <W<T> as Go>::e");
    let items = items.replace("e = X", "e = <W<T> as Go>::e");
    let (value, says) = match low {
        true => ("Lo { n: 0 }", "{}"),
        false => ("Hi { n: 0 }", "panic"),
    };
    format!(
        "{PRELUDE}\n{items}\n\nfn main() with {says} {{\n    let w = W {{ x: {value} }};\n    \
         let _v = {};\n}}\n",
        p.use_
    )
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore)]
fn within_an_undecided_projection() {
    let mut failures = Vec::new();
    let mut count = 0;
    for p in &POSITIONS {
        for (honest, body) in [(true, p.honest), (false, p.lie)] {
            for low in [true, false] {
                count += 1;
                let src = positioned(p, body, low);
                let got = verdict(&src);
                let ok = match (honest, &got) {
                    // Refused however it's called: in generic code, the
                    // projection may be none.
                    (false, Err(m)) => !m.starts_with("accepted"),
                    (true, r) => r.is_ok(),
                    _ => false,
                };
                if !ok {
                    let what = format!(
                        "{}, {}, of {}",
                        p.name,
                        if honest { "keeping it" } else { "breaking it" },
                        if low { "Lo" } else { "Hi" },
                    );
                    failures.push(format!("{what}: {got:?}\n{src}"));
                }
            }
        }
    }
    assert!(count >= 28, "only {count} programs");
    assert!(
        failures.is_empty(),
        "{} of {count} failed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

/// The type an impl of `Go` with `effect e = panic unless T: Low` is for:
/// its header, the value of `T` in it, and a value of it.
struct Header {
    name: &'static str,
    params: &'static str,
    head: &'static str,
    /// `T`'s value, of `self`.
    value: &'static str,
    /// A value of the header with `T` = `ARG`, a value of it.
    make: &'static str,
}

const HEADERS: [Header; 6] = [
    Header {
        name: "a struct",
        params: "<T: Val>",
        head: "V<T>",
        value: "self.x.value()",
        make: "V { x: ARG }",
    },
    Header {
        name: "a reference",
        params: "<'a, T: Val>",
        head: "&'a T",
        value: "(**self).value()",
        make: "&ARG",
    },
    Header {
        name: "a tuple",
        params: "<T: Val>",
        head: "(T, u8)",
        value: "self.0.value()",
        make: "(ARG, 1u8)",
    },
    Header {
        name: "an array",
        params: "<T: Val>",
        head: "[T; 1]",
        value: "self[0].value()",
        make: "[ARG]",
    },
    Header {
        name: "an option",
        params: "<T: Val>",
        head: "Option<T>",
        value: "(match self { Some(t) => t.value(), None => 0 })",
        make: "Some(ARG)",
    },
    Header {
        name: "a struct in a struct",
        params: "<T: Val>",
        head: "V<V<T>>",
        value: "self.x.x.value()",
        make: "V { x: V { x: ARG } }",
    },
];

/// How the type the impl is called at is `Low`, if it is.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Given {
    /// By an impl for it alone: `Lo`.
    Instance,
    /// By a blanket impl, for every `Bx<U>` whose `U` is `Foo`.
    Blanket,
    /// Not at all: `Hi`.
    Never,
}

/// How the impl's method says its effects.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Spelling {
    /// `with Self::e`.
    SelfE,
    /// `with panic unless T: Low`.
    Spelled,
    /// `with panic`, which is more than the trait's method has where `T`
    /// is `Low`: always refused.
    Plain,
}

/// A program with `Go`'s impl for header `h`, whose method says its
/// effects as `spelled` says and adds `add` to the value, called of a type
/// that's `Low` as `g` says, by a `main` that says `panic` where it isn't.
fn headed(h: &Header, g: Given, spelled: Spelling, add: u8) -> String {
    let arg = match g {
        Given::Instance => "Lo { n: 0 }",
        Given::Blanket => "Bx { n: 254, u: A { n: 0 } }",
        Given::Never => "Hi { n: 0 }",
    };
    let says = match g {
        Given::Never => "panic",
        _ => "{}",
    };
    let with = match spelled {
        Spelling::SelfE => "Self::e",
        Spelling::Spelled => "panic unless T: Low",
        Spelling::Plain => "panic",
    };
    format!(
        "trait Val {{\n    fn value(&self) -> u8;\n}}\n\n\
         trait Low: Val {{\n    law fn value(&self) -> {{ r: u8 | r < 255 }};\n}}\n\n\
         trait Foo {{}}\n\n\
         #[derive(Discard)]\nstruct A {{ n: u8 }}\n\nimpl Foo for A {{}}\n\n\
         #[derive(Discard)]\nstruct Lo {{ n: u8 }}\n\n\
         #[derive(Discard)]\nstruct Hi {{ n: u8 }}\n\n\
         impl Val for Lo {{\n    fn value(&self) -> u8 {{ 254 }}\n}}\n\n\
         impl Low for Lo {{}}\n\n\
         impl Val for Hi {{\n    fn value(&self) -> u8 {{ 3 }}\n}}\n\n\
         #[derive(Discard where U: Discard)]\nstruct Bx<U> {{ n: {{ v: u8 | v < 255 }}, u: U }}\n\n\
         impl<U> Val for Bx<U> {{\n    fn value(&self) -> u8 {{ self.n }}\n}}\n\n\
         impl<U: Foo> Low for Bx<U> {{}}\n\n\
         #[derive(Discard where T: Discard)]\nstruct V<T> {{ x: T }}\n\n\
         trait Go {{\n    effect e;\n\n    fn go(&self) -> u8 with Self::e;\n}}\n\n\
         impl{} Go for {} {{\n    effect e = panic unless T: Low;\n\n    \
         fn go(&self) -> u8 with {with} {{ {} + {add} }}\n}}\n\n\
         fn main() with {says} {{\n    let v = {};\n    let _r = Go::go(&v);\n}}\n",
        h.params,
        h.head,
        h.value,
        h.make.replace("ARG", arg),
    )
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore)]
fn guarded_impls_by_header() {
    let mut failures = Vec::new();
    let mut count = 0;
    for h in &HEADERS {
        for g in [Given::Instance, Given::Blanket, Given::Never] {
            for spelled in [Spelling::SelfE, Spelling::Spelled, Spelling::Plain] {
                for add in [1u8, 2] {
                    count += 1;
                    let src = headed(h, g, spelled, add);
                    let got = verdict(&src);
                    let ok = match (spelled, add, &got) {
                        // More than the trait's method has where `T: Low`.
                        (Spelling::Plain, _, Err(m)) => m.contains("can't there"),
                        // Two more overflows where `T: Low`: refused.
                        (_, 2, Err(m)) => !m.starts_with("accepted"),
                        (Spelling::SelfE | Spelling::Spelled, 1, r) => r.is_ok(),
                        _ => false,
                    };
                    if !ok {
                        let what =
                            format!("{}, `Low` given {g:?}, {spelled:?}, adding {add}", h.name);
                        failures.push(format!("{what}: {got:?}\n{src}"));
                    }
                }
            }
        }
    }
    assert!(count >= 108, "only {count} programs");
    assert!(
        failures.is_empty(),
        "{} of {count} failed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}
