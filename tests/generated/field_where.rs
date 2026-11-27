//! Deterministic families for a struct's field's refinement that hangs on
//! a bound (docs/93-invariants-where.md): `Kept<T>`'s `n` is at most `100`
//! and at most its `of`'s `cap`, where `T: Cap`. A body has a `Kept` at
//! each kind of type the bound can be asked of — a type parameter with the
//! bound and without, a type that meets it and one that doesn't, a type
//! only partly known, the type behind an `impl Trait`, an associated type —
//! and makes or changes it each way a value can be: a literal, a field
//! written at a place, through a reference, in another struct, in an
//! array, in a variant, lent to a call, by a closure, by `swap`, in a
//! loop. Each keeps the refinement or breaks it. (`swap` with a variable
//! that has no refinement is refused of any refined field, kept or not: a
//! function generic in what it writes may write any value it's given.)
//!
//! Each is refused, or accepted and run by the interpreter, of a type that
//! meets the bound and of one that doesn't, where the type is the
//! caller's: the interpreter checks a struct's refinements, those that
//! hold of the copy's types, whenever one is made, written or read. So a
//! body that breaks it and is let through is caught in the copy where the
//! bound holds. And what's known is asked too: a body that counts on the
//! refinement is accepted only where it knows the bound. They're about
//! what programs mean, so they run on Linux alone.

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
        Err(_) => Err("accepted, but the interpreter found a refinement broken".to_string()),
    }
}

/// `Lo` meets the bound, with a `cap` of 9, and `Hi` doesn't; `Wrap<T>`
/// does where `T` does; `SrcLo`'s `Out` is `Lo` and `SrcHi`'s is `Hi`; and
/// `hidden` gives a `Lo` nobody can name.
const PRELUDE: &str = r#"use core::mem::swap;

trait Thing {
    fn id(&self) -> u8;
}

trait Cap {
    #[measure]
    fn cap(&self) -> u8;
}

#[derive(Discard)]
struct Lo { c: u8 }

impl Thing for Lo {
    fn id(&self) -> u8 { 1 }
}

impl Cap for Lo {
    #[measure]
    fn cap(&self) -> u8 { self.c }
}

#[derive(Discard)]
struct Hi {}

impl Thing for Hi {
    fn id(&self) -> u8 { 2 }
}

#[derive(Discard where T: Discard)]
struct Wrap<T> { inner: T }

impl<T: Thing> Thing for Wrap<T> {
    fn id(&self) -> u8 { self.inner.id() }
}

impl<T: Cap> Cap for Wrap<T> {
    #[measure]
    fn cap(&self) -> u8 { self.inner.cap() }
}

trait Src {
    type Out: Thing + Discard;

    fn out(&self) -> Self::Out;
}

#[derive(Discard)]
struct SrcLo {}

impl Src for SrcLo {
    type Out = Lo;

    fn out(&self) -> Lo { Lo { c: 9 } }
}

#[derive(Discard)]
struct SrcHi {}

impl Src for SrcHi {
    type Out = Hi;

    fn out(&self) -> Hi { Hi {} }
}

fn hidden() -> impl Thing + Discard { Lo { c: 9 } }

#[derive(Discard where T: Discard)]
struct Kept<T> { of: T, n: { v: u8 | v <= 100 && v <= of.cap() where T: Cap } }

#[derive(Discard where T: Discard)]
struct Outer<T> { inner: Kept<T> }

fn put0(r: &mut u8) ensures *r == 0 {
    *r = 0;
}

fn put200(r: &mut u8) ensures *r == 200 {
    *r = 200;
}
"#;

/// The kind of type a body has a `Kept` at.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Instance {
    /// A type parameter with no such bound: of a `Lo` and of a `Hi`.
    Param,
    /// One with the bound.
    Bound,
    /// `Lo`, which meets it.
    Meets,
    /// `Hi`, which doesn't, and never will.
    Never,
    /// `Wrap<T>`, of any `T`: of a `Lo` and of a `Hi`.
    Partly,
    /// `Wrap<Hi>`, which doesn't either.
    PartlyNever,
    /// What `hidden` gives, which is a `Lo`.
    Opaque,
    /// `S::Out`: of a `SrcLo` and of a `SrcHi`.
    Proj,
}

const INSTANCES: [Instance; 8] = [
    Instance::Param,
    Instance::Bound,
    Instance::Meets,
    Instance::Never,
    Instance::Partly,
    Instance::PartlyNever,
    Instance::Opaque,
    Instance::Proj,
];

impl Instance {
    /// Is the bound known to hold in the body?
    fn known(self) -> bool {
        matches!(self, Instance::Bound | Instance::Meets)
    }

    /// Can it hold of no copy of the body?
    fn never(self) -> bool {
        matches!(self, Instance::Never | Instance::PartlyNever)
    }

    /// The head of `way`, up to its body, which has `of`, of the type.
    fn head(self, effects: &str) -> String {
        let (generics, params, of) = match self {
            Instance::Param => ("<T: Thing + Discard>", "of: T", ""),
            Instance::Bound => ("<T: Thing + Cap + Discard>", "of: T", ""),
            Instance::Meets => ("", "of: Lo", ""),
            Instance::Never => ("", "of: Hi", ""),
            Instance::Partly => (
                "<T: Thing + Discard>",
                "x: T",
                "    let of = Wrap { inner: x };\n",
            ),
            Instance::PartlyNever => ("", "x: Hi", "    let of = Wrap { inner: x };\n"),
            Instance::Opaque => ("", "", "    let of = hidden();\n"),
            Instance::Proj => ("<S: Src>", "s: &S", "    let of = s.out();\n"),
        };
        format!("fn way{generics}({params}) -> u8{effects} {{\n{of}")
    }

    /// The calls of `way` in `main`.
    fn calls(self) -> &'static str {
        match self {
            Instance::Param => "    let _a = way(Lo { c: 9 });\n    let _b = way(Hi {});\n",
            Instance::Bound | Instance::Meets => "    let _a = way(Lo { c: 9 });\n",
            Instance::Never | Instance::PartlyNever => "    let _a = way(Hi {});\n",
            Instance::Partly => "    let _a = way(Lo { c: 9 });\n    let _b = way(Hi {});\n",
            Instance::Opaque => "    let _a = way();\n",
            Instance::Proj => "    let _a = way(&SrcLo {});\n    let _b = way(&SrcHi {});\n",
        }
    }
}

/// A way a `Kept` is made or changed, with `V` for what its `n` ends as,
/// and then read.
const WAYS: [(&str, &str); 10] = [
    ("literal", "    let k = Kept { of, n: V };\n    k.n\n"),
    (
        "assign",
        "    let mut k = Kept { of, n: 0 };\n    k.n = V;\n    k.n\n",
    ),
    (
        "reference",
        "    let mut k = Kept { of, n: 0 };\n    let r = &mut k;\n    r.n = V;\n    k.n\n",
    ),
    (
        "nested",
        "    let mut o = Outer { inner: Kept { of, n: 0 } };\n    o.inner.n = V;\n    o.inner.n\n",
    ),
    (
        "array",
        "    let mut a = [Kept { of, n: 0 }];\n    a[0].n = V;\n    a[0].n\n",
    ),
    (
        "variant",
        "    let mut o = Some(Kept { of, n: 0 });\n    match &mut o {\n        Some(k) => k.n = V,\n        None => {},\n    }\n    match o {\n        Some(k) => k.n,\n        None => 0,\n    }\n",
    ),
    (
        "lend",
        "    let mut k = Kept { of, n: 0 };\n    putV(&mut k.n);\n    k.n\n",
    ),
    (
        "closure",
        "    let mut k = Kept { of, n: 0 };\n    let mut f = || k.n = V;\n    f();\n    k.n\n",
    ),
    (
        "swap",
        "    let mut k = Kept { of, n: 0 };\n    let mut x: u8 = V;\n    swap(&mut k.n, &mut x);\n    k.n\n",
    ),
    (
        "loop",
        "    let mut k = Kept { of, n: 0 };\n    let mut i: u8 = 0;\n    while i < 1 {\n        k.n = V;\n        i = i + 1;\n    }\n    k.n\n",
    ),
];

fn program(i: Instance, body: &str, effects: &str) -> String {
    format!(
        "{PRELUDE}\n{}{body}}}\n\nfn main() with panic {{\n{}}}\n",
        i.head(effects),
        i.calls()
    )
}

/// Every way of making or changing one, at every kind of type, keeping
/// the refinement or breaking it.
#[test]
#[cfg_attr(not(target_os = "linux"), ignore)]
fn a_refinement_where_a_bound_holds_is_kept() {
    let mut count = 0;
    for i in INSTANCES {
        for (name, way) in WAYS {
            for honest in [true, false] {
                let v = if honest { "0" } else { "200" };
                let src = program(i, &way.replace('V', v), " with panic");
                let got = verdict(&src);
                count += 1;
                let what = format!("{i:?} {name} honest={honest}");
                if let Err(e) = &got {
                    assert!(!e.starts_with("accepted, but"), "{what}: {e}\n{src}");
                }
                // One that keeps it is accepted, wherever it is; one that
                // breaks it only where the bound can't hold.
                match (honest, i.never()) {
                    (true, false) if name == "swap" => {
                        let e = got.expect_err(&format!("{what}: accepted\n{src}"));
                        assert!(e.contains("of what `swap` may write"), "{what}: {e}\n{src}");
                    }
                    (true, _) | (false, true) => {
                        assert!(got.is_ok(), "{what}: refused: {got:?}\n{src}")
                    }
                    (false, false) => {
                        let e = got.expect_err(&format!("{what}: accepted\n{src}"));
                        assert!(
                            e.contains("can't prove `v <= 100 && v <= of.cap()`"),
                            "{what}: {e}\n{src}"
                        );
                    }
                }
            }
        }
    }
    assert_eq!(count, 160);
}

/// A body that counts on the refinement, `n + 100` with no `panic`, is
/// accepted only where it knows the bound holds: not of a type parameter
/// without it, nor of a type that may meet it in some copy.
#[test]
#[cfg_attr(not(target_os = "linux"), ignore)]
fn a_refinement_where_a_bound_holds_is_known_only_there() {
    for i in INSTANCES {
        let body = "    let k = Kept { of, n: 0 };\n    k.n + 100\n";
        // What the walk knows of a value it just made is the value, so
        // the struct comes from a function that says nothing of it.
        let body = body.replace("Kept { of, n: 0 }", "fresh(of)");
        let src = program(i, &body, " with {}").replace(
            "fn way",
            "fn fresh<U>(of: U) -> Kept<U> { Kept { of, n: 0 } }\n\nfn way",
        );
        let got = verdict(&src);
        if let Err(e) = &got {
            assert!(!e.starts_with("accepted, but"), "{i:?}: {e}\n{src}");
        }
        match i.known() {
            true => assert!(got.is_ok(), "{i:?}: refused: {got:?}\n{src}"),
            false => {
                let e = got.expect_err(&format!("{i:?}: accepted\n{src}"));
                assert!(e.contains("`+` may panic"), "{i:?}: {e}\n{src}");
            }
        }
    }
}
