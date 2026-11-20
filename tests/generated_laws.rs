//! Deterministic families for laws (docs/88-iterator-laws.md): a trait
//! `S`'s law of method `f` of trait `P`, a supertrait or one its `where`
//! names, in each shape a contract can take — a refined result, `ensures`,
//! a result's type argument, a measure of `S`'s — and each thing the impl
//! of `P` may write differently from the law — a `mut` parameter it names
//! or doesn't, `mut self`, a renamed parameter, the method's default body
//! or its own — by an impl for a struct of its own, a generic one, an
//! adapter whose body counts on its field's law, one whose impl of `S`
//! needs more than its impl of `P`, one for fewer types than `P`'s, and a
//! blanket impl of `P`; each keeping the law or breaking it,
//! called through `S`, through `P` alone, and at the known type. Each is
//! refused, or accepted and run by the interpreter, which checks every
//! law at each call of an impl's method. And a law of a method of a trait
//! with glue, for each kind of type whose impl of it is derived or built
//! in, beside the same law of a written impl at each (`laws_of_glue`,
//! retrospective 16). They're about what programs mean, so they run on
//! Linux alone.

use t_lang::error::Phase;

/// Checks `src` and, if it's accepted, runs it: it must finish, having
/// broken no refinement. Gives the error's message if it's refused.
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
        Err(_) => Err("accepted, but the interpreter found a broken refinement".to_string()),
    }
}

/// What the law says, and what the impl of `P` writes.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Shape {
    /// A refined result, naming a parameter.
    Result,
    /// `ensures`, of what's behind a `&mut` parameter.
    Ensures,
    /// A refinement of the result's type argument.
    Option,
    /// A refined result naming a measure of `S`'s.
    Measure,
    /// A law of a method of a trait `S` names in its `where`.
    Conditional,
    /// A `mut` parameter the law's result names.
    MutResult,
    /// A `mut` parameter the law doesn't name.
    MutUnnamed,
    /// `mut self`, which the law names by its measure.
    MutSelf,
    /// The parameter the law names, by another name.
    Renamed,
    /// The method's default body, which the impl doesn't replace.
    DefaultKept,
    /// The impl's own body for a method with a default.
    DefaultOverride,
}

/// The rule against a `mut` parameter a law names.
const MUT: &str = "can't be `mut` in `f`";
/// The rule refusing a pair of impls it can't check.
const UNCHECKED: &str = "can't check that";
/// What a call that doesn't know the law can't prove.
const UNPROVED: &str = "can't prove";

/// A shape's declarations: `P`'s method (with a default body or not), the
/// law, and the methods of an impl of `P` for a struct with a field `n`
/// keeping the law, for an adapter of another implementing `S` (field
/// `a`) keeping it by its field's, and breaking it; then the use of `T`,
/// an expression of `u8` that indexes `a`, two bytes long.
struct Decls {
    method: &'static str,
    law: &'static str,
    own: &'static str,
    adapter: Option<&'static str>,
    lie: Option<&'static str>,
    call: &'static str,
}

fn decls(s: Shape) -> Decls {
    const F: &str = "    fn f(&self, k: usize) -> usize;\n";
    const LAW: &str = "    law fn f(&self, k: usize) -> { n: usize | n <= k };\n";
    const LIE: &str = "    fn f(&self, k: usize) -> usize { let _ = k; 100 }\n";
    const DELEGATE: &str = "    fn f(&self, k: usize) -> usize { self.a.f(k) }\n";
    match s {
        Shape::Result | Shape::Conditional => Decls {
            method: F,
            law: LAW,
            own: "    fn f(&self, k: usize) -> usize { k / 2 }\n",
            adapter: Some(DELEGATE),
            lie: Some(LIE),
            call: "a[T.f(1)]",
        },
        Shape::Ensures => Decls {
            method: "    fn f(&self, k: usize, r: &mut usize);\n",
            law: "    law fn f(&self, k: usize, r: &mut usize) ensures *r <= k;\n",
            own: "    fn f(&self, k: usize, r: &mut usize) { *r = k; }\n",
            adapter: Some("    fn f(&self, k: usize, r: &mut usize) { self.a.f(k, r) }\n"),
            lie: Some("    fn f(&self, k: usize, r: &mut usize) { let _ = k; *r = 100; }\n"),
            call: "{ let mut v: usize = 0; T.f(1, &mut v); a[v] }",
        },
        Shape::Option => Decls {
            method: "    fn f(&self, k: usize) -> Option<usize>;\n",
            law: "    law fn f(&self, k: usize) -> Option<{ n: usize | n <= k }>;\n",
            own: "    fn f(&self, k: usize) -> Option<usize> { Some(k / 2) }\n",
            adapter: Some("    fn f(&self, k: usize) -> Option<usize> { self.a.f(k) }\n"),
            lie: Some("    fn f(&self, k: usize) -> Option<usize> { let _ = k; Some(100) }\n"),
            call: "match T.f(1) { Some(n) => a[n], None => 0 }",
        },
        Shape::Measure => Decls {
            method: "    fn f(&self) -> usize;\n",
            law: "    law fn f(&self) -> { n: usize | n <= self.size() };\n",
            own: "    fn f(&self) -> usize { self.n }\n",
            adapter: Some("    fn f(&self) -> usize { self.a.f() }\n"),
            lie: Some("    fn f(&self) -> usize { 100 }\n"),
            call: "if T.size() < 2 { a[T.f()] } else { 0 }",
        },
        Shape::MutResult => Decls {
            method: F,
            law: LAW,
            own: "    fn f(&self, mut k: usize) -> usize { k = k / 2; k }\n",
            adapter: None,
            lie: Some("    fn f(&self, mut k: usize) -> usize { k = 100; k }\n"),
            call: "a[T.f(1)]",
        },
        Shape::MutUnnamed => Decls {
            method: "    fn f(&self, k: usize, j: usize) -> usize;\n",
            law: "    law fn f(&self, k: usize, j: usize) -> { n: usize | n <= k };\n",
            own: "    fn f(&self, k: usize, mut j: usize) -> usize { j = 100; let _ = j; k }\n",
            adapter: None,
            lie: Some(
                "    fn f(&self, k: usize, mut j: usize) -> usize { let _ = k; j = 100; j }\n",
            ),
            call: "a[T.f(1, 5)]",
        },
        Shape::MutSelf => Decls {
            method: "    fn f(self) -> usize;\n",
            law: "    law fn f(self) -> { n: usize | n <= self.size() };\n",
            own: "    fn f(mut self) -> usize { self.n = 0; self.n }\n",
            adapter: None,
            lie: Some("    fn f(mut self) -> usize { self.n = 100; self.n }\n"),
            call: "a[T.f()]",
        },
        Shape::Renamed => Decls {
            method: F,
            law: LAW,
            own: "    fn f(&self, j: usize) -> usize { j / 2 }\n",
            adapter: Some("    fn f(&self, j: usize) -> usize { self.a.f(j) }\n"),
            lie: Some("    fn f(&self, j: usize) -> usize { let _ = j; 100 }\n"),
            call: "a[T.f(1)]",
        },
        Shape::DefaultKept => Decls {
            method: "    fn f(&self, k: usize) -> usize { k / 2 }\n",
            law: LAW,
            own: "",
            adapter: Some(""),
            lie: None,
            call: "a[T.f(1)]",
        },
        Shape::DefaultOverride => Decls {
            method: "    fn f(&self, k: usize) -> usize { k / 2 }\n",
            law: LAW,
            own: "    fn f(&self, k: usize) -> usize { k / 3 }\n",
            adapter: Some(DELEGATE),
            lie: Some(LIE),
            call: "a[T.f(1)]",
        },
    }
}

/// How `P` and `S` are implemented.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Impl {
    /// For a struct of its own.
    Own,
    /// For a generic struct, at any `A`.
    Generic,
    /// For a generic struct whose field `a` implements both, whose own
    /// `f` counts on its field's law.
    Adapter,
    /// For a generic struct, `S` with a bound `P`'s doesn't have.
    Bounded,
    /// `P` for every `X<A>`, `S` only for `X<u8>`.
    Specific,
    /// `P` for any type of another trait's, `S` for a struct.
    Blanket,
}

/// How `f` is called.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Caller {
    /// Generically, knowing `T: S` (and `T: P`).
    ThroughS,
    /// Generically, knowing only `T: P`.
    ThroughP,
    /// At the known type.
    Known,
}

/// The program: `s`'s traits, implemented as `i` with `methods`, called
/// by `c`.
fn program(s: Shape, i: Impl, methods: &str, c: Caller) -> String {
    let d = decls(s);
    let conditional = s == Shape::Conditional;
    let measure = "    #[measure]\n    fn size(&self) -> usize;\n";
    let mut src = format!("trait P {{\n{}}}\n\n", d.method);
    let law = match conditional {
        true => d.law.replace(";\n", "\n    where\n        Self: P;\n"),
        false => d.law.to_string(),
    };
    let supers = if conditional { "" } else { ": P" };
    src.push_str(&format!("trait S{supers} {{\n{measure}\n{law}}}\n\n"));
    let size = |body: &str| format!("    #[measure]\n    fn size(&self) -> usize {{ {body} }}\n");
    let make = match i {
        Impl::Own => {
            src.push_str("#[derive(Discard)]\nstruct X { n: usize }\n\n");
            src.push_str(&format!("impl P for X {{\n{methods}}}\n\n"));
            src.push_str(&format!("impl S for X {{\n{}}}\n\n", size("self.n")));
            "X { n: 1 }"
        }
        Impl::Generic => {
            src.push_str("#[derive(Discard)]\nstruct X<A> { n: usize, a: A }\n\n");
            src.push_str(&format!("impl<A: Discard> P for X<A> {{\n{methods}}}\n\n"));
            src.push_str(&format!(
                "impl<A: Discard> S for X<A> {{\n{}}}\n\n",
                size("self.n")
            ));
            "X { n: 1, a: 0u8 }"
        }
        Impl::Bounded => {
            src.push_str("#[derive(Discard)]\nstruct X<A> { n: usize, a: A }\n\n");
            src.push_str(&format!("impl<A: Discard> P for X<A> {{\n{methods}}}\n\n"));
            src.push_str(&format!(
                "impl<A: Copy + Discard> S for X<A> {{\n{}}}\n\n",
                size("self.n")
            ));
            "X { n: 1, a: 0u8 }"
        }
        Impl::Adapter => {
            // The field's type keeps the law, as an own struct's would.
            src.push_str("#[derive(Discard)]\nstruct Y { n: usize }\n\n");
            src.push_str(&format!("impl P for Y {{\n{}}}\n\n", d.own));
            src.push_str(&format!("impl S for Y {{\n{}}}\n\n", size("self.n")));
            src.push_str("#[derive(Discard)]\nstruct X<A> { n: usize, a: A }\n\n");
            src.push_str(&format!(
                "impl<A: P + Discard> P for X<A> {{\n{methods}}}\n\n"
            ));
            let bounds = if conditional { "S + P" } else { "S" };
            src.push_str(&format!(
                "impl<A: {bounds} + Discard> S for X<A> {{\n{}}}\n\n",
                size("self.a.size()")
            ));
            "X { n: 1, a: Y { n: 1 } }"
        }
        Impl::Specific => {
            src.push_str("#[derive(Discard)]\nstruct X<A> { n: usize, a: A }\n\n");
            src.push_str(&format!("impl<A: Discard> P for X<A> {{\n{methods}}}\n\n"));
            src.push_str(&format!("impl S for X<u8> {{\n{}}}\n\n", size("self.n")));
            "X { n: 1, a: 0u8 }"
        }
        Impl::Blanket => {
            src.push_str("trait Q {\n    fn q(&self) -> usize;\n}\n\n");
            src.push_str("#[derive(Discard)]\nstruct X { n: usize }\n\n");
            src.push_str("impl Q for X {\n    fn q(&self) -> usize { self.n }\n}\n\n");
            src.push_str(&format!("impl<A: Q + Discard> P for A {{\n{methods}}}\n\n"));
            src.push_str(&format!("impl S for X {{\n{}}}\n\n", size("self.n")));
            "X { n: 1 }"
        }
    };
    let by_value = s == Shape::MutSelf;
    match c {
        Caller::ThroughS | Caller::ThroughP => {
            let bound = match (c, conditional) {
                (Caller::ThroughP, _) => "P",
                (_, true) => "S + P",
                (_, false) => "S",
            };
            let call = d.call.replace('T', "t");
            // `f` takes `self` by value, of a value whose measure is small.
            let (bound, param, arg) = match by_value {
                true => (
                    format!("{bound} + S + Discard"),
                    "t: { t: T | t.size() < 2 }".to_string(),
                    make.to_string(),
                ),
                false => (bound.to_string(), "t: &T".to_string(), format!("&{make}")),
            };
            src.push_str(&format!(
                "fn g<T: {bound}>({param}) -> u8 {{\n    let a = [7u8; 2];\n    {call}\n}}\n\n\
                 fn main() with {{}} {{\n    let _ = g({arg});\n}}\n"
            ));
        }
        Caller::Known => {
            let call = d.call.replace('T', "x");
            src.push_str(&format!(
                "fn main() with {{}} {{\n    let x = {make};\n    let a = [7u8; 2];\n    let _ = {call};\n}}\n"
            ));
        }
    }
    src
}

/// What's expected of a program whose impl keeps the law: accepted, or
/// refused by a rule with this message.
fn honest(s: Shape, i: Impl, c: Caller) -> Option<&'static str> {
    // The impl of `P` writes `f`, for a pair to check.
    let writes = s != Shape::DefaultKept;
    if writes && matches!(i, Impl::Specific | Impl::Blanket) {
        return Some(UNCHECKED);
    }
    if matches!(s, Shape::MutResult | Shape::MutSelf) {
        return Some(MUT);
    }
    // Through `P` alone, the law isn't known, nor `S`'s measure.
    if c == Caller::ThroughP {
        return Some(match s {
            Shape::Measure => "has no method `size`",
            _ => UNPROVED,
        });
    }
    None
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore)]
fn laws() {
    let shapes = [
        Shape::Result,
        Shape::Ensures,
        Shape::Option,
        Shape::Measure,
        Shape::Conditional,
        Shape::MutResult,
        Shape::MutUnnamed,
        Shape::MutSelf,
        Shape::Renamed,
        Shape::DefaultKept,
        Shape::DefaultOverride,
    ];
    let impls = [
        Impl::Own,
        Impl::Generic,
        Impl::Bounded,
        Impl::Adapter,
        Impl::Specific,
        Impl::Blanket,
    ];
    let callers = [Caller::ThroughS, Caller::ThroughP, Caller::Known];
    let mut failures = Vec::new();
    let mut count = 0;
    for s in shapes {
        let d = decls(s);
        for i in impls {
            // A blanket impl can't read a field, nor an adapter's be
            // written of one that has none.
            let reads_n = matches!(s, Shape::Measure | Shape::MutSelf);
            if (reads_n && i == Impl::Blanket) || (i == Impl::Adapter && d.adapter.is_none()) {
                continue;
            }
            // The specific impl's `S`, at the known type, is of `X<u8>`;
            // generically it's of any `T: S`, and `X<u8>` is one.
            let keeps = match i {
                Impl::Adapter => d.adapter.expect("checked"),
                _ => d.own,
            };
            for (honest_, methods) in [(true, Some(keeps)), (false, d.lie)] {
                let Some(methods) = methods else {
                    continue;
                };
                for c in callers {
                    count += 1;
                    let src = program(s, i, methods, c);
                    let what = format!(
                        "{s:?} by {i:?} impls that {}, called {c:?}",
                        if honest_ { "keep the law" } else { "break it" },
                    );
                    match (verdict(&src), honest_, honest(s, i, c)) {
                        (Ok(()), true, None) => {}
                        (Err(m), true, Some(why)) if m.contains(why) => {}
                        // A lie is refused, whatever says so.
                        (Err(m), false, _) if !m.starts_with("accepted") => {}
                        (Ok(()), ..) => failures.push(format!("{what}: accepted\n{src}")),
                        (Err(m), ..) => failures.push(format!("{what}: {m}\n{src}")),
                    }
                }
            }
        }
    }
    assert!(count > 150, "only {count} programs");
    assert!(
        failures.is_empty(),
        "{} of {count} failed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

/// The types of each kind a trait's method can come from at a type: a
/// derive, the language's own rules, a written impl (retrospective 16):
/// each with a value, and whether it needs `alloc`.
const PROVIDERS: &[(&str, &str, &str)] = &[
    ("derived struct", "X", "X { a: 1 }"),
    ("derived enum", "E", "E::A"),
    ("derived generic", "W<u8>", "W { a: 1u8 }"),
    ("derived in core", "Option<u8>", "Some(1u8)"),
    ("tuple", "(u8, bool)", "(1u8, true)"),
    ("unit", "()", "()"),
    ("array", "[u8; 2]", "[1u8, 2]"),
    ("array of a struct", "[X; 1]", "[X { a: 1 }]"),
    ("integer", "u8", "1u8"),
    ("bool", "bool", "true"),
    ("char", "char", "'c'"),
    ("reference", "&'a u8", "&1u8"),
    ("reference to a struct", "&'a X", "&X { a: 1 }"),
    ("string", "&'a str", "\"s\""),
    ("box", "Box<u8>", "Box::new(1u8)"),
];

const GLUE_TYPES: &str = "#[derive(Eq, Debug, Discard)]
struct X { a: u8 }

#[derive(Eq, Debug, Discard, Copy)]
enum E { A, B }

#[derive(Eq, Debug, Discard)]
struct W<T> { a: T }
";

/// `impl S for ty {}`, with the lifetime it names.
fn impl_for(ty: &str, body: &str) -> String {
    let params = if ty.contains("'a") { "<'a>" } else { "" };
    format!("impl{params} S for {ty} {{{body}}}\n")
}

/// A program for a law of `Eq`'s `eq` (`conditional`: in a trait whose
/// `where` names `Eq`, else in a subtrait of it), kept by type `ty`, whose
/// `Eq` is a derive's or the language's; called through the law's trait
/// (`generic`) or at `ty`.
fn glue_program(ty: &str, value: &str, conditional: bool, generic: bool) -> String {
    let (law, body) = match conditional {
        false => (
            "trait S: Eq {\n    law fn eq(&self, other: &Self) -> { b: bool | !b };\n}\n",
            "",
        ),
        true => (
            "trait S {\n    #[measure]\n    fn k(&self) -> usize;\n\n    law fn eq(&self, other: \
             &Self) -> { b: bool | !b } where Self: Eq;\n}\n",
            "\n    #[measure]\n    fn k(&self) -> usize { 0 }\n",
        ),
    };
    let alloc = ty.contains("Box");
    let call = if generic {
        "same(&x, &x)"
    } else {
        "Eq::eq(&x, &x)"
    };
    format!(
        "{}{law}\n{GLUE_TYPES}\n{}\nfn same<T: S + Eq>(a: &T, b: &T) -> {{ r: bool | !r }} {{ \
         a.eq(b) }}\n\nfn main() with {{{}}} {{\n    let x = {value};\n    let arr = [0u8; 1];\n    \
         let i: usize = if {call} {{ 7 }} else {{ 0 }};\n    let _ = arr[i];\n}}\n",
        if alloc {
            "use alloc::boxed::Box;\n"
        } else {
            ""
        },
        impl_for(ty, body),
        if alloc { "alloc" } else { "" },
    )
}

/// The same law, of a method of a trait of the program's, whose impl at
/// `ty` is written: it's checked, kept (`honest`) or not.
fn written_program(ty: &str, value: &str, honest: bool, generic: bool) -> String {
    let alloc = ty.contains("Box");
    let params = if ty.contains("'a") { "<'a>" } else { "" };
    let call = if generic {
        "same(&x, &x)"
    } else {
        "MyEq::eq(&x, &x)"
    };
    format!(
        "{}trait MyEq {{\n    fn eq(&self, other: &Self) -> bool;\n}}\n\ntrait S: MyEq {{\n    \
         law fn eq(&self, other: &Self) -> {{ b: bool | !b }};\n}}\n\n{GLUE_TYPES}\n\
         impl{params} MyEq for {ty} {{\n    fn eq(&self, other: &Self) -> bool {{ let _ = other; \
         {} }}\n}}\n\n{}\nfn same<T: S>(a: &T, b: &T) -> {{ r: bool | !r }} {{ a.eq(b) }}\n\n\
         fn main() with {{{}}} {{\n    let x = {value};\n    let arr = [0u8; 1];\n    let i: usize \
         = if {call} {{ 7 }} else {{ 0 }};\n    let _ = arr[i];\n}}\n",
        if alloc {
            "use alloc::boxed::Box;\n"
        } else {
            ""
        },
        if honest { "false" } else { "true" },
        impl_for(ty, ""),
        if alloc { "alloc" } else { "" },
    )
}

/// A law of a method of a trait with glue (a derived or built-in impl,
/// whose body monomorphisation writes) is refused, for every kind of type
/// that has one; the same law of a written impl's method, for the same
/// types, is checked against its body (H1 of retrospective 16).
#[test]
#[cfg_attr(not(target_os = "linux"), ignore)]
fn laws_of_glue() {
    const GLUE: &str = "a law can't be of `Eq`'s `eq`";
    let mut failures = Vec::new();
    let mut count = 0;
    for &(what, ty, value) in PROVIDERS {
        for generic in [true, false] {
            let called = if generic {
                "through the law's trait"
            } else {
                "at the type"
            };
            for conditional in [false, true] {
                count += 1;
                let src = glue_program(ty, value, conditional, generic);
                match verdict(&src) {
                    Err(m) if m.starts_with(GLUE) => {}
                    Ok(()) => failures.push(format!("{what}, {called}: accepted\n{src}")),
                    Err(m) => failures.push(format!("{what}, {called}: {m}\n{src}")),
                }
            }
            for honest in [true, false] {
                count += 1;
                let src = written_program(ty, value, honest, generic);
                match (verdict(&src), honest) {
                    (Ok(()), true) => {}
                    (Err(m), false) if m.contains(UNPROVED) => {}
                    (Ok(()), false) => {
                        failures.push(format!("{what}, {called}: a lie accepted\n{src}"))
                    }
                    (Err(m), _) => failures.push(format!("{what}, {called}: {m}\n{src}")),
                }
            }
        }
    }
    assert!(count >= 120, "only {count} programs");
    assert!(
        failures.is_empty(),
        "{} of {count} failed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}
