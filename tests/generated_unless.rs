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

/// The type an impl of `Bump` is for, in place of a default that says
/// `with panic unless Self: Small`: its header, an instance of it that's
/// `Small` where an impl says so, and the items it needs.
struct Header {
    name: &'static str,
    params: &'static str,
    head: &'static str,
    items: &'static str,
    /// An instance of `head`, and a value of it.
    instance: &'static str,
    value: &'static str,
    /// `head` with a parameter `U: Foo` for a conditional blanket impl
    /// of `Small`, if there's one to write: `(params, head)`.
    blanket: Option<(&'static str, &'static str)>,
    /// Whether what decides who may implement a trait for it is a
    /// parameter, under references (`head_of`): then the package of
    /// whatever it's given may write one, `impl Small for &X`, so it may
    /// be `Small` whatever impls there are here.
    headless: bool,
}

const HEADERS: [Header; 7] = [
    Header {
        name: "a struct",
        params: "<T>",
        head: "W<T>",
        items: "#[derive(Discard where T: Discard)]\nstruct W<T> { n: u8, t: T }\n",
        instance: "W<u8>",
        value: "W { n: 9, t: 0u8 }",
        blanket: Some(("<U: Foo>", "W<U>")),
        headless: false,
    },
    Header {
        name: "a reference",
        params: "<'a, T>",
        head: "&'a T",
        items: "",
        instance: "&'a A",
        value: "&A { n: 9 }",
        blanket: Some(("<'a, U: Foo>", "&'a U")),
        headless: true,
    },
    Header {
        name: "a tuple",
        params: "<T>",
        head: "(T, u8)",
        items: "",
        instance: "(A, u8)",
        value: "(A { n: 9 }, 1u8)",
        blanket: Some(("<U: Foo>", "(U, u8)")),
        headless: false,
    },
    Header {
        name: "an array of a const length",
        params: "<const N: usize>",
        head: "[u8; N]",
        items: "",
        instance: "[u8; 3]",
        value: "[1u8, 2, 3]",
        blanket: None,
        headless: false,
    },
    Header {
        name: "an array",
        params: "<T>",
        head: "[T; 2]",
        items: "",
        instance: "[A; 2]",
        value: "[A { n: 9 }, A { n: 8 }]",
        blanket: Some(("<U: Foo>", "[U; 2]")),
        headless: false,
    },
    Header {
        name: "an option",
        params: "<T>",
        head: "Option<T>",
        items: "",
        instance: "Option<A>",
        value: "Some(A { n: 9 })",
        blanket: Some(("<U: Foo>", "Option<U>")),
        headless: false,
    },
    Header {
        name: "a struct in a struct",
        params: "<T>",
        head: "W<W<T>>",
        items: "#[derive(Discard where T: Discard)]\nstruct W<T> { n: u8, t: T }\n",
        instance: "W<W<u8>>",
        value: "W { n: 9, t: W { n: 8, t: 0u8 } }",
        blanket: Some(("<U: Foo>", "W<W<U>>")),
        headless: false,
    },
];

/// How `Small`'s impl for an instance of the header is given, if it is.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Given {
    /// For the instance alone.
    Instance,
    /// For every instance whose parameter is `Foo`, as `A` and `u8` are.
    Blanket,
    /// Not at all: no instance can be `Small`.
    Never,
}

/// A program with `Bump`'s impl for `h`, whose own `bump` panics if
/// `lie`, `Small`'s impl given as `g`, and a `main` calling `bump` of
/// `h`'s value, saying `panic` if `says`.
fn overridden(h: &Header, g: Given, lie: bool, says: bool) -> Option<String> {
    let small = match g {
        Given::Instance => {
            let params = if h.instance.contains("'a") {
                "<'a>"
            } else {
                ""
            };
            format!("impl{params} Small for {} {{}}\n", h.instance)
        }
        Given::Blanket => {
            let (params, head) = h.blanket?;
            format!("impl{params} Small for {head} {{}}\n")
        }
        Given::Never => String::new(),
    };
    let body = match lie {
        true => "panic(\"bumped\")",
        false => "1",
    };
    let says = if says { "panic" } else { "{}" };
    Some(format!(
        "trait Small {{}}\n\ntrait Foo {{}}\n\n#[derive(Discard)]\nstruct A {{ n: u8 }}\n\n\
         impl Foo for A {{}}\n\nimpl Foo for u8 {{}}\n\n{}\n{small}\n\
         trait Bump {{\n    fn bump(&self) -> u8 with panic unless Self: Small {{ 1 }}\n}}\n\n\
         impl{} Bump for {} {{\n    fn bump(&self) -> u8 with panic {{ {body} }}\n}}\n\n\
         fn main() with {says} {{\n    let v = {};\n    let _b = Bump::bump(&v);\n}}\n",
        h.items, h.params, h.head, h.value,
    ))
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore)]
fn overrides_by_header() {
    let mut failures = Vec::new();
    let mut count = 0;
    for h in &HEADERS {
        for g in [Given::Instance, Given::Blanket, Given::Never] {
            for lie in [false, true] {
                for says in [false, true] {
                    let Some(src) = overridden(h, g, lie, says) else {
                        continue;
                    };
                    count += 1;
                    let got = verdict(&src);
                    // Barred: some instance may be `Small`, so the impl's
                    // own method can't panic, as a call at one has no
                    // `panic`. Met: the instance called is.
                    let barred = g != Given::Never || h.headless;
                    let met = g != Given::Never;
                    let ok = match (lie, &got) {
                        (true, Err(m)) if barred => !m.starts_with("accepted"),
                        // Where none can be, it may; and every call says it.
                        (true, Err(m)) if !says => m.contains("doesn't say `with panic`"),
                        (true, Err(m)) => m.starts_with("accepted, but it failed"),
                        (false, r) if met || says => r.is_ok(),
                        (false, Err(m)) => m.contains("doesn't say `with panic`"),
                        _ => false,
                    };
                    if !ok {
                        let what = format!(
                            "{}, `Small` given {g:?}, {}, main {}",
                            h.name,
                            if lie { "panicking" } else { "not panicking" },
                            if says {
                                "saying `panic`"
                            } else {
                                "saying nothing"
                            },
                        );
                        failures.push(format!("{what}: {got:?}\n{src}"));
                    }
                }
            }
        }
    }
    assert!(count >= 80, "only {count} programs");
    assert!(
        failures.is_empty(),
        "{} of {count} failed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

/// A way a site in `h`'s body may panic without saying `panic`: through an
/// associated effect that a bound says is within `{panic}`. `params` and
/// `args` are `h`'s own, beside `T`'s or `Self`'s; `values`, what a call
/// gives them; `own`, the associated effect `h` may say for itself.
struct Route {
    name: &'static str,
    params: &'static str,
    args: &'static str,
    values: &'static str,
    body: &'static str,
    own: Option<&'static str>,
}

const ROUTES: [Route; 5] = [
    Route {
        name: "a parameter's associated effect, bounded",
        params: "I: Iterator<Item = u64, e = {panic}> + Discard",
        args: "mut it: I",
        values: "Down { n: 0 }",
        body: "let _v = it.next(); it.discard(); 1",
        own: Some("I::e"),
    },
    Route {
        name: "one bounded by another's",
        params: "J: Iterator<Item = u64, e = {panic}> + Discard, \
                 I: Iterator<Item = u64, e = J::e> + Discard",
        args: "j: J, mut it: I",
        values: "Down { n: 0 }, Down { n: 0 }",
        body: "j.discard(); let _v = it.next(); it.discard(); 1",
        own: Some("I::e"),
    },
    Route {
        name: "an opaque type's, bounded",
        params: "",
        args: "",
        values: "",
        body: "let mut it = down(); let _v = it.next(); it.discard(); 1",
        own: None,
    },
    Route {
        name: "an opaque type's, bounded by a parameter's",
        params: "J: Iterator<Item = u64, e = {panic}> + Discard",
        args: "j: J",
        values: "Down { n: 0 }",
        body: "let mut it = wrap(j); let _v = it.next(); it.discard(); 1",
        own: Some("J::e"),
    },
    Route {
        name: "a type's own",
        params: "",
        args: "",
        values: "",
        body: "let mut it = Down { n: 0 }; let _v = it.next(); it.discard(); 1",
        own: None,
    },
];

/// What every route's program has: an iterator whose `next` panics, and
/// opaque ones of it.
const ROUTE_ITEMS: &str = r#"#[derive(Discard)]
struct Down { n: u64 }

impl Iterator for Down {
    type Item = u64;
    effect e = panic;

    fn next(&mut self) -> Option<u64> with panic { panic("down") }
}

fn down() -> impl Iterator<Item = u64, e = {panic}> + Discard { Down { n: 0 } }

fn wrap<J: Iterator<Item = u64, e = {panic}> + Discard>(j: J) -> impl Iterator<Item = u64, e = J::e> + Discard { j }
"#;

/// `h` taking the route `r`, saying `with says`, provided as `p` says, and
/// a `main` calling it of `Lo`, which meets the bound, saying `main_says`.
fn routed(r: &Route, p: Provider, says: &str, main_says: &str) -> String {
    let comma = |s: &str| match s {
        "" => String::new(),
        s => format!(", {s}"),
    };
    let (params, args) = (comma(r.params), comma(r.args));
    let own = match r.params {
        "" => String::new(),
        ps => format!("<{ps}>"),
    };
    match p {
        Provider::Free => format!(
            "fn h<T: Val{params}>(x: &T{args}) -> u8 with {says} {{ {} }}\n\n\
             fn main() with {main_says} {{\n    let _v = h(&Lo {{ n: 0 }}{});\n}}",
            r.body,
            comma(r.values),
        ),
        Provider::Inherent => format!(
            "#[derive(Discard where T: Discard)]\nstruct W<T> {{ x: T }}\n\nimpl<T: Val> W<T> {{\n    \
             fn h{own}(&self{args}) -> u8 with {says} {{ {} }}\n}}\n\n\
             fn main() with {main_says} {{\n    let w = W {{ x: Lo {{ n: 0 }} }};\n    \
             let _v = w.h({});\n}}",
            r.body, r.values,
        ),
        _ => format!(
            "trait H: Val {{\n    fn h{own}(&self{args}) -> u8 with {} {{ {} }}\n}}\n\n\
             impl H for Lo {{}}\n\n\
             fn main() with {main_says} {{\n    let _v = (Lo {{ n: 0 }}).h({});\n}}",
            says.replace("T: Low", "Self: Low"),
            r.body,
            r.values,
        ),
    }
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore)]
fn sites_through_associated_effects() {
    let mut failures = Vec::new();
    let mut count = 0;
    for r in &ROUTES {
        for p in [Provider::Free, Provider::Inherent, Provider::Default] {
            // Breaking what `h` says, under the bound; keeping it by saying
            // `panic` always; and by saying the associated effect itself,
            // which `h` has whatever the bound, where it can be said.
            let mut cases = vec![
                (false, "panic unless T: Low".to_string(), "{}"),
                (true, "panic".to_string(), "panic"),
            ];
            if let Some(own) = r.own {
                cases.push((true, format!("{{{own}, panic unless T: Low}}"), "panic"));
            }
            for (honest, says, main_says) in cases {
                count += 1;
                let src = format!(
                    "{PRELUDE}\n{ROUTE_ITEMS}\n{}\n",
                    routed(r, p, &says, main_says)
                );
                let got = verdict(&src);
                // Accepted, the call panics: as `main` says it may.
                let ok = match (honest, &got) {
                    (false, Err(m)) => !m.starts_with("accepted"),
                    (true, Err(m)) => m.starts_with("accepted, but it failed"),
                    _ => false,
                };
                if !ok {
                    let what = format!("{}, {p:?}, saying `with {says}`", r.name);
                    failures.push(format!("{what}: {got:?}\n{src}"));
                }
            }
        }
    }
    assert!(count >= 39, "only {count} programs");
    assert!(
        failures.is_empty(),
        "{} of {count} failed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}
