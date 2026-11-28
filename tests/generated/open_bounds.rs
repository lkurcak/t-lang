//! Deterministic families for `Fn` bounds that leave what calls do open
//! (docs/104-open-fn-bounds.md): a bound in a function, an impl, a trait's
//! method, an opaque result, and another package's exported function;
//! open, `with {}`, `with panic`, `with alloc`, or `with e` of an effect
//! parameter; in a function whose effects are inferred, that says them
//! right, that's exported and says them, or that says none; given a
//! closure that does nothing, may panic, allocates, or both; called from a
//! `main` whose effects are inferred or that says them.
//!
//! A model says which are refused: a bound takes what's within it, and an
//! open one anything; calls of what it bounds do what the bound says, and
//! an open one's or an effect parameter's what the closure does; a function
//! that says its effects must say those; a caller must say what the call
//! does, and no `with` can name an open opaque result's. Each program the
//! model accepts is run by the interpreter, which checks at every
//! allocation that each function being run says `alloc`, with its effect
//! arguments: the closures that allocate do. They're about what programs
//! mean, so they run on Linux alone.

use t_lang::error::Phase;

/// Checks `main` (with `lib` as package `lib1`, if there is one) and, if
/// it's accepted, runs it. Gives the error's message if it's refused, or
/// what went wrong running it.
fn verdict(main: &str, lib: Option<&str>) -> Result<(), String> {
    let read = |p: &std::path::Path| -> std::io::Result<String> {
        match (p.to_str(), lib) {
            (Some("t.toml"), Some(_)) => Ok(
                "libraries = [\"alloc\"]\n\n[dependencies]\nlib1 = { path = \"lib1\" }\n"
                    .to_string(),
            ),
            (Some("lib1/lib.t"), Some(lib)) => Ok(lib.to_string()),
            _ => t_lang::no_files(p),
        }
    };
    let checked = t_lang::check_files(std::path::Path::new("main.t"), main, &read);
    let program = match checked.result {
        Ok(p) => p,
        Err(e) => {
            assert_eq!(
                e.phase,
                Phase::Type,
                "{}\n{main}\n{}",
                lib.unwrap_or(""),
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

/// A set of effects, of `alloc` and `panic`.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Fx {
    alloc: bool,
    panic: bool,
}

impl Fx {
    const NONE: Fx = Fx {
        alloc: false,
        panic: false,
    };

    fn within(self, other: Fx) -> bool {
        (!self.alloc || other.alloc) && (!self.panic || other.panic)
    }

    /// As a `with` says it.
    fn text(self) -> &'static str {
        match (self.alloc, self.panic) {
            (false, false) => "{}",
            (true, false) => "alloc",
            (false, true) => "panic",
            (true, true) => "{alloc, panic}",
        }
    }
}

/// What a bound says of its calls.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Bound {
    /// No `with`: open, `F::e`.
    Open,
    /// `with X`.
    At(Fx),
    /// `with e`, of an effect parameter `e`.
    Param,
}

const BOUNDS: [Bound; 5] = [
    Bound::Open,
    Bound::At(Fx::NONE),
    Bound::At(Fx {
        alloc: false,
        panic: true,
    }),
    Bound::At(Fx {
        alloc: true,
        panic: false,
    }),
    Bound::Param,
];

impl Bound {
    /// What's after the signature in the bound.
    fn with(self) -> String {
        match self {
            Bound::Open => String::new(),
            Bound::At(x) => format!(" with {}", x.text()),
            Bound::Param => " with e".to_string(),
        }
    }

    /// The effect parameter it needs declared, if it does.
    fn param(self) -> &'static str {
        match self {
            Bound::Param => "effect e, ",
            _ => "",
        }
    }

    /// What a `with` says its calls do in, with the bound on `ty`.
    fn calls(self, ty: &str) -> String {
        match self {
            Bound::Open => format!("{ty}::e"),
            Bound::At(x) => x.text().to_string(),
            Bound::Param => "e".to_string(),
        }
    }

    /// What a `with` that says its calls do and may also panic says, with
    /// the bound on `ty`.
    fn calls_and_panic(self, ty: &str) -> String {
        match self {
            Bound::At(x) => Fx { panic: true, ..x }.text().to_string(),
            b => format!("{{panic, {}}}", b.calls(ty)),
        }
    }

    /// Does it take a closure that does `a`?
    fn takes(self, a: Fx) -> bool {
        match self {
            Bound::Open | Bound::Param => true,
            Bound::At(x) => a.within(x),
        }
    }

    /// What a call of what it bounds does, given a closure that does `a`,
    /// as a caller sees it.
    fn does(self, a: Fx) -> Fx {
        match self {
            Bound::Open | Bound::Param => a,
            Bound::At(x) => x,
        }
    }

    /// Does a body whose calls of what it bounds are all it does do nothing,
    /// as its function sees it?
    fn inside_none(self) -> bool {
        self == Bound::At(Fx::NONE)
    }
}

/// How the function with the bound says its effects.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Own {
    /// It doesn't, and isn't exported: inferred.
    Inferred,
    /// It says what its calls do.
    Says,
    /// It's exported, and says what its calls do.
    Exported,
    /// It says `with {}`.
    Wrong,
}

/// A closure given to the bound, and what it does.
struct Arg {
    text: &'static str,
    does: Fx,
}

const ARGS: [Arg; 4] = [
    Arg {
        text: "|| 7u8",
        does: Fx::NONE,
    },
    Arg {
        text: "|| opaque() + 250",
        does: Fx {
            alloc: false,
            panic: true,
        },
    },
    Arg {
        text: "|| {\n        let b = Box::new(3u8);\n        *b\n    }",
        does: Fx {
            alloc: true,
            panic: false,
        },
    },
    Arg {
        text: "|| {\n        let b = Box::new(opaque());\n        *b + 250\n    }",
        does: Fx {
            alloc: true,
            panic: true,
        },
    },
];

/// What `main` says it does, if it says.
const CALLERS: [Option<Fx>; 5] = [
    None,
    Some(Fx::NONE),
    Some(Fx {
        alloc: false,
        panic: true,
    }),
    Some(Fx {
        alloc: true,
        panic: false,
    }),
    Some(Fx {
        alloc: true,
        panic: true,
    }),
];

const PRELUDE: &str = "use alloc::boxed::Box;

fn opaque() -> u8 { 1 }
";

/// `main`'s `with`, or none.
fn says(c: Option<Fx>) -> String {
    c.map_or(String::new(), |c| format!(" with {}", c.text()))
}

/// What `own` makes the function with the bound say, with the bound on
/// `ty`.
fn own_with(own: Own, b: Bound, ty: &str) -> String {
    match own {
        Own::Inferred => String::new(),
        Own::Says | Own::Exported => format!(" with {}", b.calls(ty)),
        Own::Wrong => " with {}".to_string(),
    }
}

/// Whether the model accepts a program whose function with bound `b` says
/// its effects as `own`, given a closure that does `a`, called by a `main`
/// that says `c`.
fn accepts(b: Bound, own: Own, a: Fx, c: Option<Fx>) -> bool {
    if !b.takes(a) {
        return false;
    }
    if own == Own::Wrong && !b.inside_none() {
        return false;
    }
    let call = match own {
        Own::Wrong => Fx::NONE,
        _ => b.does(a),
    };
    c.is_none_or(|c| call.within(c))
}

/// Runs each case, and fails with every one the model and the checker or
/// the interpreter disagree on.
fn run_all(cases: Vec<(String, Option<String>, bool)>) {
    let mut wrong = Vec::new();
    let n = cases.len();
    for (main, lib, accept) in cases {
        let got = verdict(&main, lib.as_deref());
        let ok = match (&got, accept) {
            (Ok(()), true) => true,
            (Err(e), false) => !e.starts_with("accepted"),
            _ => false,
        };
        if !ok {
            wrong.push(format!(
                "{}{main}\nexpected {}, got {got:?}\n",
                lib.map(|l| format!("{l}\n// main.t:\n"))
                    .unwrap_or_default(),
                if accept { "it to run" } else { "an error" },
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {n} wrong:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// The bound on a function's own type parameter.
#[test]
#[cfg_attr(not(target_os = "linux"), ignore)]
fn in_a_function() {
    let mut cases = Vec::new();
    for b in BOUNDS {
        for own in [Own::Inferred, Own::Says, Own::Exported, Own::Wrong] {
            for a in &ARGS {
                for c in CALLERS {
                    let vis = if own == Own::Exported { "export " } else { "" };
                    let main = format!(
                        "{PRELUDE}\n{vis}fn h<{}F: Fn() -> u8{} + Discard>(f: F) -> u8{} {{ f() }}\n\n\
                         fn main(){} {{\n    let _v = h({});\n}}\n",
                        b.param(),
                        b.with(),
                        own_with(own, b, "F"),
                        says(c),
                        a.text
                    );
                    cases.push((main, None, accepts(b, own, a.does, c)));
                }
            }
        }
    }
    run_all(cases);
}

/// The bound on an impl's type parameter, of its own method.
#[test]
#[cfg_attr(not(target_os = "linux"), ignore)]
fn in_an_impl() {
    let mut cases = Vec::new();
    for b in BOUNDS {
        for own in [Own::Inferred, Own::Says, Own::Wrong] {
            for a in &ARGS {
                for c in CALLERS {
                    let main = format!(
                        "{PRELUDE}\n#[derive(Discard where F: Discard)]\nstruct S<F> {{ f: F }}\n\n\
                         impl<{}F: Fn() -> u8{}> S<F> {{\n    fn get(&self) -> u8{} {{ (self.f)() }}\n}}\n\n\
                         fn main(){} {{\n    let s = S {{ f: {} }};\n    let _v = s.get();\n}}\n",
                        b.param(),
                        b.with(),
                        own_with(own, b, "F"),
                        says(c),
                        a.text
                    );
                    cases.push((main, None, accepts(b, own, a.does, c)));
                }
            }
        }
    }
    run_all(cases);
}

/// The bound on a trait's method's type parameter, kept by its impl, called
/// on the impl's type and through a type parameter.
#[test]
#[cfg_attr(not(target_os = "linux"), ignore)]
fn in_a_trait_method() {
    let mut cases = Vec::new();
    for b in BOUNDS {
        for own in [Own::Says, Own::Wrong] {
            for a in &ARGS {
                for c in CALLERS {
                    for generic in [false, true] {
                        let call = match generic {
                            false => format!("    let _v = U {{}}.call({});\n", a.text),
                            true => "    let _v = go(&U {});\n".to_string(),
                        };
                        let go = match generic {
                            false => String::new(),
                            true => {
                                format!("fn go<T: Run>(t: &T) -> u8 {{ t.call({}) }}\n\n", a.text)
                            }
                        };
                        let main = format!(
                            "{PRELUDE}\ntrait Run {{\n    fn call<{p}F: FnMut() -> u8{w} + Discard>(&self, f: F) -> u8{};\n}}\n\n\
                             #[derive(Discard)]\nstruct U {{}}\n\n\
                             impl Run for U {{\n    fn call<{p}G: FnMut() -> u8{w} + Discard>(&self, mut g: G) -> u8{} {{ g() }}\n}}\n\n\
                             {go}fn main(){} {{\n{call}}}\n",
                            own_with(own, b, "F"),
                            own_with(own, b, "G"),
                            says(c),
                            p = b.param(),
                            w = b.with(),
                        );
                        cases.push((main, None, accepts(b, own, a.does, c)));
                    }
                }
            }
        }
    }
    run_all(cases);
}

/// The bound of an opaque result, which no caller's `with` can name if
/// it's open.
#[test]
#[cfg_attr(not(target_os = "linux"), ignore)]
fn in_an_opaque_result() {
    let mut cases = Vec::new();
    for b in &BOUNDS[..4] {
        for a in &ARGS {
            for c in CALLERS {
                let main = format!(
                    "{PRELUDE}\nfn mk() -> impl Fn() -> u8{} + Discard {{ {} }}\n\n\
                     fn main(){} {{\n    let f = mk();\n    let _v = f();\n}}\n",
                    b.with(),
                    a.text,
                    says(c)
                );
                let accept = b.takes(a.does)
                    && match (b, c) {
                        (_, None) => true,
                        (Bound::Open, Some(_)) => false,
                        (b, Some(c)) => b.does(a.does).within(c),
                    };
                cases.push((main, None, accept));
            }
        }
    }
    run_all(cases);
}

/// The bound of another package's exported function, which says its
/// effects (docs/24-visibility.md, E4).
#[test]
#[cfg_attr(not(target_os = "linux"), ignore)]
fn in_another_package() {
    let mut cases = Vec::new();
    for b in BOUNDS {
        for own in [Own::Exported, Own::Wrong] {
            let lib = format!(
                "export fn h<{}F: Fn() -> u8{} + Discard>(f: F) -> u8{} {{ f() }}\n",
                b.param(),
                b.with(),
                own_with(own, b, "F")
            );
            for a in &ARGS {
                for c in CALLERS {
                    let main = format!(
                        "{PRELUDE}\nfn main(){} {{\n    let _v = lib1::h({});\n}}\n",
                        says(c),
                        a.text
                    );
                    cases.push((main, Some(lib.clone()), accepts(b, own, a.does, c)));
                }
            }
        }
    }
    run_all(cases);
}

/// What an impl's method inherits of its trait's (the inheritance family of
/// retrospective 15): the trait's bound and the impl's, each of every kind,
/// the impl's type parameter named apart and its closure `mut`, saying all
/// the trait's method says or less; called on the impl's type and through a
/// type parameter. Only an impl whose bound is the trait's is accepted
/// (lemma O5 of docs/104-open-fn-bounds.md).
#[test]
#[cfg_attr(not(target_os = "linux"), ignore)]
fn inherited_by_an_impl() {
    let mut cases = Vec::new();
    for want in BOUNDS {
        for got in BOUNDS {
            for less in [false, true] {
                for a in &ARGS {
                    for generic in [false, true] {
                        // The trait's says it may also panic; the impl's
                        // says that too, or only what its calls do.
                        let trait_with = format!(" with {}", want.calls_and_panic("F"));
                        let impl_with = match less {
                            false => format!(" with {}", got.calls_and_panic("G")),
                            true => format!(" with {}", got.calls("G")),
                        };
                        let (call, go) = match generic {
                            false => (
                                format!("    let _v = U {{}}.call({});\n", a.text),
                                String::new(),
                            ),
                            true => (
                                "    let _v = go(&U {});\n".to_string(),
                                format!("fn go<T: Run>(t: &T) -> u8 {{ t.call({}) }}\n\n", a.text),
                            ),
                        };
                        let main = format!(
                            "{PRELUDE}\ntrait Run {{\n    fn call<{}F: FnMut() -> u8{} + Discard>(&self, f: F) -> u8{trait_with};\n}}\n\n\
                             #[derive(Discard)]\nstruct U {{}}\n\n\
                             impl Run for U {{\n    fn call<{}G: FnMut() -> u8{} + Discard>(&self, mut g: G) -> u8{impl_with} {{ g() }}\n}}\n\n\
                             {go}fn main() {{\n{call}}}\n",
                            want.param(),
                            want.with(),
                            got.param(),
                            got.with(),
                        );
                        cases.push((main, None, want == got && want.takes(a.does)));
                    }
                }
            }
        }
    }
    run_all(cases);
}
