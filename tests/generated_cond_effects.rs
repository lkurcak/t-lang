//! Deterministic families for conditional effects
//! (docs/90-conditional-effects.md): a function `h` saying `with panic if
//! P` or `with alloc if P`, in each shape a site in its body can take — a
//! `panic(..)` in a branch, an operator, a call with a condition of its
//! own, a `mut` parameter changed before the panic, `unwrap` of an
//! `Option`, what's behind a `&mut`, `push` — each keeping its condition or
//! breaking it; called where the condition is ruled out, where nothing's
//! known, where the caller says the effect, under a branch that rules it
//! out, through a function value, and in a closure that must have no
//! effects. Each is refused, or accepted and run by the interpreter, which
//! checks at every panic and allocation that each function being run says
//! it, with its condition holding where it was called. They're about what
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

/// Where `h` is called.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Caller {
    /// Where its condition is ruled out, by a literal or a refinement.
    Known,
    /// Where nothing's known of what it's given.
    Unknown,
    /// Where the caller says the effect, and it happens.
    Says,
    /// Under a branch that rules the condition out.
    Branch,
    /// Through a function value the walk knows is `h`: a call of `h`
    /// (docs/78-callable-values.md).
    Value,
    /// Given to a function that calls it with what it's given, which has
    /// every effect `h` says.
    Given,
    /// In a closure given to a bound with no effects.
    Closure,
}

const CALLERS: [Caller; 7] = [
    Caller::Known,
    Caller::Unknown,
    Caller::Says,
    Caller::Branch,
    Caller::Value,
    Caller::Given,
    Caller::Closure,
];

/// One shape of `h`: its effect, its body keeping its condition and
/// breaking it, and its callers, if it can have that one.
struct Shape {
    name: &'static str,
    atom: &'static str,
    honest: &'static str,
    lie: &'static str,
    caller: fn(Caller) -> Option<&'static str>,
}

const PRELUDE: &str = "use alloc::list::List;

fn opaque() -> u8 { 9 }

fn opaque_o() -> Option<u8> { None }

fn opaque_l() -> List<u8> { List::new() }

fn call<F: Fn() -> u8 + Discard>(f: F) -> u8 { f() }

fn apply(f: fn(u8) -> u8 with panic, x: u8) -> u8 with panic { f(x) }

fn apply_o(f: fn(Option<u8>) -> u8 with panic, x: Option<u8>) -> u8 with panic { f(x) }
";

/// Callers of an `h` taking a `u8`, which panics if it's more than 3.
fn by_u8(c: Caller) -> Option<&'static str> {
    Some(match c {
        Caller::Known => "fn main() with {} {\n    let _v = h(3);\n}\n",
        Caller::Unknown => "fn main() with {} {\n    let a = opaque();\n    let _v = h(a);\n}\n",
        Caller::Says => "fn main() with panic {\n    let _v = h(9);\n}\n",
        Caller::Branch => {
            "fn main() with {} {\n    let a = opaque();\n    if a <= 3 {\n        let _v = h(a);\n    }\n}\n"
        }
        Caller::Value => "fn main() with {} {\n    let f = h;\n    let _v = f(3);\n}\n",
        Caller::Given => "fn main() with {} {\n    let _v = apply(h, 3);\n}\n",
        Caller::Closure => "fn main() with {} {\n    let _v = call(|| h(3));\n}\n",
    })
}

/// Callers of an `h` taking an `Option<u8>`, which panics if it's `None`.
fn by_option(c: Caller) -> Option<&'static str> {
    Some(match c {
        Caller::Known => "fn main() with {} {\n    let _v = h(Some(9));\n}\n",
        Caller::Unknown => "fn main() with {} {\n    let a = opaque_o();\n    let _v = h(a);\n}\n",
        Caller::Says => "fn main() with panic {\n    let _v = h(None);\n}\n",
        Caller::Branch => {
            "fn main() with {} {\n    let a = opaque_o();\n    if a.is_some() {\n        let _v = h(a);\n    }\n}\n"
        }
        Caller::Value => "fn main() with {} {\n    let f = h;\n    let _v = f(Some(9));\n}\n",
        Caller::Given => "fn main() with {} {\n    let _v = apply_o(h, Some(9));\n}\n",
        Caller::Closure => "fn main() with {} {\n    let _v = call(|| h(Some(9)));\n}\n",
    })
}

/// Callers of an `h` taking a `&mut List<u8>`, which panics if it's empty.
fn by_list(c: Caller) -> Option<&'static str> {
    Some(match c {
        Caller::Known => {
            "fn main() with alloc {\n    let mut l = List::new();\n    l.push(9);\n    let _v = h(&mut l);\n}\n"
        }
        Caller::Unknown => {
            "fn main() with alloc {\n    let mut l = opaque_l();\n    let _v = h(&mut l);\n}\n"
        }
        Caller::Says => {
            "fn main() with {alloc, panic} {\n    let mut l: List<u8> = List::new();\n    let _v = h(&mut l);\n}\n"
        }
        Caller::Branch => {
            "fn main() with alloc {\n    let mut l = opaque_l();\n    if l.len() > 0 {\n        let _v = h(&mut l);\n    }\n}\n"
        }
        Caller::Value => {
            "fn main() with alloc {\n    let mut l = List::new();\n    l.push(9);\n    let f = h;\n    let _v = f(&mut l);\n}\n"
        }
        Caller::Given | Caller::Closure => return None,
    })
}

/// Callers of an `h` pushing to a `&mut List<u8>`, which allocates if
/// there's no room.
fn by_room(c: Caller) -> Option<&'static str> {
    Some(match c {
        Caller::Known => {
            "fn run(l: { l: List<u8> | l.len() == 0 && l.capacity() == 4 }) -> List<u8> with {} {\n    let mut m = l;\n    h(&mut m);\n    m\n}\n\nfn main() with alloc {\n    let _l = run(List::with_capacity(4));\n}\n"
        }
        Caller::Unknown => {
            "fn run(l: List<u8>) -> List<u8> with {} {\n    let mut m = l;\n    h(&mut m);\n    m\n}\n\nfn main() with alloc {\n    let _l = run(List::new());\n}\n"
        }
        Caller::Says => {
            "fn main() with alloc {\n    let mut l: List<u8> = List::new();\n    h(&mut l);\n}\n"
        }
        Caller::Branch => {
            "fn run(l: List<u8>) -> List<u8> with {} {\n    let mut m = l;\n    if m.len() < m.capacity() {\n        h(&mut m);\n    }\n    m\n}\n\nfn main() with alloc {\n    let _l = run(List::with_capacity(4));\n}\n"
        }
        Caller::Value => {
            "fn run(l: { l: List<u8> | l.len() == 0 && l.capacity() == 4 }) -> List<u8> with {} {\n    let mut m = l;\n    let f = h;\n    f(&mut m);\n    m\n}\n\nfn main() with alloc {\n    let _l = run(List::with_capacity(4));\n}\n"
        }
        Caller::Given | Caller::Closure => return None,
    })
}

const SHAPES: &[Shape] = &[
    Shape {
        name: "a `panic` in a branch",
        atom: "panic",
        honest: "fn h(x: u8) -> u8 with panic if x > 3 {\n    if x > 3 {\n        panic(\"big\");\n    }\n    x\n}\n",
        lie: "fn h(x: u8) -> u8 with panic if x > 3 {\n    if x > 2 {\n        panic(\"big\");\n    }\n    x\n}\n",
        caller: by_u8,
    },
    Shape {
        name: "an operator",
        atom: "panic",
        honest: "fn h(x: u8) -> u8 with panic if x > 3 { x + 252 }\n",
        lie: "fn h(x: u8) -> u8 with panic if x > 3 { x + 253 }\n",
        caller: by_u8,
    },
    Shape {
        name: "a call with a condition",
        atom: "panic",
        honest: "fn g(y: u8) -> u8 with panic if y > 3 {\n    if y > 3 {\n        panic(\"big\");\n    }\n    y\n}\n\nfn h(x: u8) -> u8 with panic if x > 3 { g(x) }\n",
        lie: "fn g(y: u8) -> u8 with panic if y > 3 {\n    if y > 3 {\n        panic(\"big\");\n    }\n    y\n}\n\nfn h(x: u8) -> u8 with panic if x > 3 { g(x + 1) }\n",
        caller: by_u8,
    },
    Shape {
        name: "a `mut` parameter",
        atom: "panic",
        honest: "fn h(mut x: u8) -> u8 with panic if x > 3 {\n    if x > 3 {\n        panic(\"big\");\n    }\n    x = x * 2;\n    x\n}\n",
        lie: "fn h(mut x: u8) -> u8 with panic if x > 3 {\n    x = x + 1;\n    if x > 3 {\n        panic(\"big\");\n    }\n    x\n}\n",
        caller: by_u8,
    },
    Shape {
        name: "`unwrap` of an `Option`",
        atom: "panic",
        honest: "fn h(o: Option<u8>) -> u8 with panic if o.is_none() { o.unwrap() }\n",
        lie: "fn h(o: Option<u8>) -> u8 with panic if o.is_none() {\n    match o {\n        Some(x) => if x > 3 { panic(\"big\") } else { x },\n        None => 0,\n    }\n}\n",
        caller: by_option,
    },
    Shape {
        name: "what's behind a `&mut`",
        atom: "panic",
        honest: "fn h(l: &mut List<u8>) -> u8 with panic if l.len() == 0 { l.pop().unwrap() }\n",
        lie: "fn h(l: &mut List<u8>) -> u8 with panic if l.len() == 0 {\n    l.clear();\n    l.pop().unwrap()\n}\n",
        caller: by_list,
    },
    Shape {
        name: "`push`",
        atom: "alloc",
        honest: "fn h(l: &mut List<u8>) with alloc if l.len() == l.capacity() { l.push(1) }\n",
        lie: "fn h(l: &mut List<u8>) with alloc if l.len() == l.capacity() {\n    l.push(1);\n    l.push(2)\n}\n",
        caller: by_room,
    },
];

#[test]
#[cfg_attr(not(target_os = "linux"), ignore)]
fn conditional_effects() {
    let mut failures = Vec::new();
    let mut count = 0;
    for s in SHAPES {
        for c in CALLERS {
            let Some(caller) = (s.caller)(c) else {
                continue;
            };
            for (honest, h) in [(true, s.honest), (false, s.lie)] {
                count += 1;
                let src = format!("{PRELUDE}\n{h}\n{caller}");
                let what = format!(
                    "{} {}, called {c:?}",
                    s.name,
                    if honest {
                        "keeping its condition"
                    } else {
                        "breaking it"
                    },
                );
                let says = format!("doesn't say `with {}`", s.atom);
                let got = verdict(&src);
                let ok = match (honest, c, &got) {
                    // A body that breaks its condition is refused, however
                    // it's called.
                    (false, _, Err(m)) => !m.starts_with("accepted"),
                    (false, _, Ok(())) => false,
                    (true, Caller::Known | Caller::Branch | Caller::Value | Caller::Closure, r) => {
                        r.is_ok()
                    }
                    (true, Caller::Unknown | Caller::Given, Err(m)) => m.contains(&says),
                    // It panics, as it says it may there.
                    (true, Caller::Says, r) if s.atom == "panic" => {
                        matches!(r, Err(m) if m.starts_with("accepted, but it failed"))
                    }
                    (true, Caller::Says, r) => r.is_ok(),
                    _ => false,
                };
                if !ok {
                    failures.push(format!("{what}: {got:?}\n{src}"));
                }
            }
        }
    }
    assert!(count > 70, "only {count} programs");
    assert!(
        failures.is_empty(),
        "{} of {count} failed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}
