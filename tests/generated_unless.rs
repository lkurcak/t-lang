//! Deterministic families for effects said `unless` a bound holds
//! (docs/92-where-a-bound-holds.md): a function `h` saying `with panic
//! unless T: Low`, provided each way a body can be — a function's own, an
//! inherent method's, a trait's default, an impl's own method in a
//! default's place — with each shape a site in its body can take — an
//! operator the bound's law proves, a call that says `unless` itself, a
//! call with a condition, a `panic(..)`, a closure — each keeping what it
//! says or breaking it; called of a type that meets the bound, of one
//! that doesn't, by code generic in one that does, and in one that
//! needn't, and where the caller says the effect. Each is refused, or
//! accepted and run by the interpreter, which checks at every panic that
//! each function being run says it, in the copy that's run. Every body
//! that breaks what it says does panic of the type that meets the bound,
//! so one let through is caught there. They're about what programs mean,
//! so they run on Linux alone.

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

/// `Lo` meets the bound, and its value is the greatest the law allows, so
/// anything more than one added to it overflows. `Hi` doesn't meet it.
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

fn g<T: Val>(x: &T) -> u8 with panic unless T: Low { x.value() + 1 }

fn g2<T: Val>(x: &T) -> u8 with panic { x.value() + 2 }

fn c(v: u8) -> u8 with panic if v == 255 {
    if v == 255 {
        panic("top");
    }
    v + 1
}

fn c2(v: u8) -> u8 with panic if v >= 254 {
    if v >= 254 {
        panic("top");
    }
    v
}
"#;

/// A shape of site in `h`'s body, of `X`, a `&T`: keeping what `h` says,
/// if it can, and breaking it.
struct Site {
    name: &'static str,
    honest: Option<&'static str>,
    lie: &'static str,
}

const SITES: [Site; 5] = [
    Site {
        name: "an operator",
        honest: Some("X.value() + 1"),
        lie: "X.value() + 2",
    },
    Site {
        name: "a call that says `unless`",
        honest: Some("g(X)"),
        lie: "g2(X)",
    },
    Site {
        name: "a call with a condition",
        honest: Some("c(X.value())"),
        lie: "c2(X.value())",
    },
    Site {
        name: "a `panic(..)`",
        honest: None,
        lie: "{ let v = X.value(); if v == 254 { panic(\"top\"); } v }",
    },
    Site {
        name: "a closure",
        honest: Some("{ let f = |m: u8| m; f(X.value()) + 1 }"),
        lie: "{ let f = |m: u8| m + 2; f(X.value()) }",
    },
];

/// How `h`'s body is provided.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Provider {
    Free,
    Inherent,
    Default,
    /// An impl's own method, for `Lo`, in place of a default that can't
    /// panic.
    Override,
}

const PROVIDERS: [Provider; 4] = [
    Provider::Free,
    Provider::Inherent,
    Provider::Default,
    Provider::Override,
];

/// Where `h` is called.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Caller {
    /// Of `Lo`, which meets the bound, where no `panic` is said.
    Meets,
    /// Of `Hi`, which doesn't, where none is said.
    Lacks,
    /// By code generic in a type that meets it, where none is said.
    GenericWith,
    /// By code generic in a type that needn't, where none is said.
    GenericWithout,
    /// Of `Hi`, where the caller says `panic`.
    Says,
}

const CALLERS: [Caller; 5] = [
    Caller::Meets,
    Caller::Lacks,
    Caller::GenericWith,
    Caller::GenericWithout,
    Caller::Says,
];

/// `h` with `body`, provided as `p` says.
fn provided(p: Provider, body: &str) -> String {
    match p {
        Provider::Free => format!(
            "fn h<T: Val>(x: &T) -> u8 with panic unless T: Low {{ {} }}",
            body.replace('X', "x")
        ),
        Provider::Inherent => format!(
            "#[derive(Discard where T: Discard)]\nstruct W<T> {{ x: T }}\n\nimpl<T: Val> W<T> {{\n    \
             fn h(&self) -> u8 with panic unless T: Low {{ {} }}\n}}",
            body.replace('X', "(&self.x)")
        ),
        Provider::Default => format!(
            "trait H: Val {{\n    fn h(&self) -> u8 with panic unless Self: Low {{ {} }}\n}}\n\n\
             impl H for Lo {{}}\n\nimpl H for Hi {{}}",
            body.replace('X', "self")
        ),
        Provider::Override => format!(
            "trait H: Val {{\n    fn h(&self) -> u8 with panic unless Self: Low {{ 0 }}\n}}\n\n\
             impl H for Lo {{\n    fn h(&self) -> u8 with panic {{ {} }}\n}}\n\n\
             impl H for Hi {{\n    fn h(&self) -> u8 with panic {{ g2(self) }}\n}}",
            body.replace('X', "self")
        ),
    }
}

/// A `main` calling `h`, provided as `p` says, as `c` says.
fn caller(p: Provider, c: Caller) -> String {
    let value = match c {
        Caller::Meets | Caller::GenericWith | Caller::GenericWithout => "Lo { n: 0 }",
        Caller::Lacks | Caller::Says => "Hi { n: 0 }",
    };
    let says = match c {
        Caller::Says => "panic",
        _ => "{}",
    };
    let generic = matches!(c, Caller::GenericWith | Caller::GenericWithout);
    let bound = match (p, c) {
        (Provider::Free | Provider::Inherent, Caller::GenericWith) => "Low",
        (Provider::Free | Provider::Inherent, _) => "Val",
        (_, Caller::GenericWith) => "H + Low",
        _ => "H",
    };
    match (p, generic) {
        (Provider::Free, false) => {
            format!("fn main() with {says} {{\n    let _v = h(&{value});\n}}")
        }
        (Provider::Free, true) => format!(
            "fn call<T: {bound}>(x: &T) -> u8 with {{}} {{ h(x) }}\n\n\
             fn main() with {{}} {{\n    let _v = call(&{value});\n}}"
        ),
        (Provider::Inherent, false) => format!(
            "fn main() with {says} {{\n    let w = W {{ x: {value} }};\n    let _v = w.h();\n}}"
        ),
        (Provider::Inherent, true) => format!(
            "fn call<T: {bound}>(w: &W<T>) -> u8 with {{}} {{ w.h() }}\n\n\
             fn main() with {{}} {{\n    let w = W {{ x: {value} }};\n    let _v = call(&w);\n}}"
        ),
        (_, false) => format!("fn main() with {says} {{\n    let _v = {value}.h();\n}}"),
        (_, true) => format!(
            "fn call<T: {bound}>(x: &T) -> u8 with {{}} {{ x.h() }}\n\n\
             fn main() with {{}} {{\n    let _v = call(&{value});\n}}"
        ),
    }
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore)]
fn effects_unless_a_bound() {
    let mut failures = Vec::new();
    let mut count = 0;
    for p in PROVIDERS {
        for s in &SITES {
            for c in CALLERS {
                for (honest, body) in [(true, s.honest), (false, Some(s.lie))] {
                    let Some(body) = body else {
                        continue;
                    };
                    count += 1;
                    let src = format!("{PRELUDE}\n{}\n\n{}\n", provided(p, body), caller(p, c));
                    let what = format!(
                        "{p:?}, {} {}, called {c:?}",
                        s.name,
                        if honest {
                            "keeping what it says"
                        } else {
                            "breaking it"
                        },
                    );
                    let got = verdict(&src);
                    let ok = match (honest, c, &got) {
                        // A body that breaks what it says is refused,
                        // however it's called.
                        (false, _, Err(m)) => !m.starts_with("accepted"),
                        (false, _, Ok(())) => false,
                        (true, Caller::Meets | Caller::GenericWith | Caller::Says, r) => r.is_ok(),
                        (true, Caller::Lacks | Caller::GenericWithout, Err(m)) => {
                            m.contains("doesn't say `with panic`")
                        }
                        _ => false,
                    };
                    if !ok {
                        failures.push(format!("{what}: {got:?}\n{src}"));
                    }
                }
            }
        }
    }
    assert!(count >= 180, "only {count} programs");
    assert!(
        failures.is_empty(),
        "{} of {count} failed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}
