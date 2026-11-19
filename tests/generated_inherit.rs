//! Deterministic families for what an impl's method inherits from its
//! trait's (docs/86-trait-contracts.md): with the trait's contract held
//! fixed, everything the impl may write differently from it — a `mut`
//! parameter that its trait's result, `ensures`, result's type argument,
//! precondition or `&mut` parameter names or refines, or that none does,
//! `mut self`, a renamed parameter, an overridden default method, and its
//! own body for a default measure — for a struct, a generic struct and any
//! type of another trait, each keeping the trait's word or breaking it,
//! called generically and at the known type. Each is refused, or accepted
//! and run by the interpreter, which checks every impl's call against its
//! trait's contract. Retrospective 15 found two holes here that step 86's
//! family, which held the impl's syntax fixed, missed. They're about what
//! programs mean, so they run on Linux alone.

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

/// What the impl writes differently from its trait's method.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Shape {
    /// A `mut` parameter the trait's result names.
    MutResult,
    /// A `mut` parameter the trait's `ensures` names.
    MutEnsures,
    /// A `mut` parameter a type argument of the trait's result names.
    MutOption,
    /// A `mut` parameter the trait's contract doesn't name.
    MutUnnamed,
    /// A `mut` parameter only the trait's precondition names.
    MutPre,
    /// A `mut` parameter the trait's precondition refines, which each
    /// value given to it must keep.
    MutRefined,
    /// A `mut` parameter the refinement of a `&mut` parameter's target
    /// names, which is checked of what's written through it.
    MutTarget,
    /// `mut self`, which the trait's result names by its measure.
    MutSelf,
    /// The parameter the trait's result names, by another name.
    Renamed,
    /// That, and `mut`.
    RenamedMut,
    /// Its own body for a trait's default method with a contract.
    MethodOverride,
    /// Its own body for a trait's default measure.
    MeasureOverride,
    /// Its own body for a trait's default measure, the same as it.
    MeasureSame,
    /// Its own body for a trait's default measure, not marked a measure.
    MeasureUnmarked,
    /// No body for a trait's default measure, which it has then.
    MeasureDefault,
}

/// What a program is expected to do, for an impl keeping its trait's word.
enum Honest {
    /// Be accepted, and run.
    Accepted,
    /// Be refused anyway, by a rule saying this: inheritance's rules
    /// refuse more than lies.
    Refused(&'static str),
}

/// The rule against a `mut` parameter the trait's promises name.
const MUT: &str = "can't be `mut` in `f`";
/// The rule against an impl's own body for a default measure.
const FINAL: &str = "has a body in trait `Tr`";

/// The trait; the impl's methods keeping its word, and breaking it (if
/// there's a way); and the use of `t` (generically) or `x` (at the known
/// type), an expression of `u8` that indexes `a`, two bytes long.
fn shape(
    s: Shape,
) -> (
    &'static str,
    &'static str,
    Option<&'static str>,
    &'static str,
) {
    const LEN: &str = "    #[measure]\n    fn len(&self) -> usize;\n    #[measure]\n    fn is_empty(&self) -> bool { self.len() == 0 }\n";
    const EMPTY: &str = "if T.is_empty() { a[T.len()] } else { 0 }";
    match s {
        Shape::MutResult => (
            "    fn f(&self, k: usize) -> { n: usize | n <= k };\n",
            "    fn f(&self, mut k: usize) -> usize { k = k / 2; k }\n",
            Some("    fn f(&self, mut k: usize) -> usize { k = 100; k }\n"),
            "a[T.f(1)]",
        ),
        Shape::MutEnsures => (
            "    fn f(&self, k: usize, r: &mut usize) ensures *r <= k;\n",
            "    fn f(&self, mut k: usize, r: &mut usize) { *r = k; k = 100; let _ = k; }\n",
            Some("    fn f(&self, mut k: usize, r: &mut usize) { k = 100; *r = k; }\n"),
            "{ let mut v: usize = 0; T.f(1, &mut v); a[v] }",
        ),
        Shape::MutOption => (
            "    fn f(&self, k: usize) -> Option<{ n: usize | n <= k }>;\n",
            "    fn f(&self, mut k: usize) -> Option<usize> { k = k / 2; Some(k) }\n",
            Some("    fn f(&self, mut k: usize) -> Option<usize> { k = 100; Some(k) }\n"),
            "match T.f(1) { Some(n) => a[n], None => 0 }",
        ),
        Shape::MutUnnamed => (
            "    fn f(&self, k: usize, j: usize) -> { n: usize | n <= k };\n",
            "    fn f(&self, k: usize, mut j: usize) -> usize { j = 100; let _ = j; k }\n",
            Some("    fn f(&self, k: usize, mut j: usize) -> usize { j = 100; j }\n"),
            "a[T.f(1, 5)]",
        ),
        Shape::MutPre => (
            "    fn f(&self, k: usize, j: { j: usize | j <= k }) -> { n: usize | n < 2 };\n",
            "    fn f(&self, mut k: usize, j: usize) -> usize { k = 100; let _ = k; if j < 2 { j } else { 0 } }\n",
            // `j <= k` is of the `k` passed, 5, not of the `k` it's set to.
            Some("    fn f(&self, mut k: usize, j: usize) -> usize { k = 1; let _ = k; j }\n"),
            "a[T.f(5, 5)]",
        ),
        Shape::MutRefined => (
            "    fn f(&self, k: usize, j: { j: usize | j <= k }) -> { n: usize | n <= k };\n",
            "    fn f(&self, k: usize, mut j: usize) -> usize { if j > 0 { j = j - 1; } j }\n",
            Some("    fn f(&self, k: usize, mut j: usize) -> usize { let _ = k; j = 100; j }\n"),
            "a[T.f(1, 1)]",
        ),
        Shape::MutTarget => (
            "    fn f(&self, k: usize, r: &mut { v: usize | v <= k });\n",
            "    fn f(&self, mut k: usize, r: &mut usize) { *r = k; k = 100; let _ = k; }\n",
            Some("    fn f(&self, mut k: usize, r: &mut usize) { k = 100; *r = k; }\n"),
            "{ let mut v: { v: usize | v <= 1 } = 0; T.f(1, &mut v); a[v] }",
        ),
        Shape::MutSelf => (
            "    #[measure]\n    fn size(&self) -> usize;\n    fn f(self) -> { n: usize | n <= self.size() };\n",
            "    #[measure]\n    fn size(&self) -> usize { self.n }\n    fn f(mut self) -> usize { self.n = 0; self.n }\n",
            Some(
                "    #[measure]\n    fn size(&self) -> usize { self.n }\n    fn f(mut self) -> usize { self.n = 100; self.n }\n",
            ),
            "a[T.f()]",
        ),
        Shape::Renamed => (
            "    fn f(&self, k: usize) -> { n: usize | n <= k };\n",
            "    fn f(&self, j: usize) -> usize { j / 2 }\n",
            Some("    fn f(&self, j: usize) -> usize { let _ = j; 100 }\n"),
            "a[T.f(1)]",
        ),
        Shape::RenamedMut => (
            "    fn f(&self, k: usize) -> { n: usize | n <= k };\n",
            "    fn f(&self, mut j: usize) -> usize { j = j / 2; j }\n",
            Some("    fn f(&self, mut j: usize) -> usize { j = 100; j }\n"),
            "a[T.f(1)]",
        ),
        Shape::MethodOverride => (
            "    fn f(&self, k: usize) -> { n: usize | n <= k } { k / 2 }\n",
            "    fn f(&self, k: usize) -> usize { let _ = k; 0 }\n",
            Some("    fn f(&self, k: usize) -> usize { let _ = k; 100 }\n"),
            "a[T.f(1)]",
        ),
        Shape::MeasureOverride => (
            LEN,
            "    #[measure]\n    fn len(&self) -> usize { 5 }\n    #[measure]\n    fn is_empty(&self) -> bool { false }\n",
            Some(
                "    #[measure]\n    fn len(&self) -> usize { 5 }\n    #[measure]\n    fn is_empty(&self) -> bool { true }\n",
            ),
            EMPTY,
        ),
        Shape::MeasureSame => (
            LEN,
            "    #[measure]\n    fn len(&self) -> usize { 0 }\n    #[measure]\n    fn is_empty(&self) -> bool { self.len() == 0 }\n",
            None,
            EMPTY,
        ),
        Shape::MeasureUnmarked => (
            LEN,
            "    #[measure]\n    fn len(&self) -> usize { 5 }\n    fn is_empty(&self) -> bool { false }\n",
            Some(
                "    #[measure]\n    fn len(&self) -> usize { 5 }\n    fn is_empty(&self) -> bool { true }\n",
            ),
            EMPTY,
        ),
        Shape::MeasureDefault => (
            LEN,
            "    #[measure]\n    fn len(&self) -> usize { 0 }\n",
            None,
            EMPTY,
        ),
    }
}

/// What's expected of `s`'s impl that keeps its trait's word.
fn honest(s: Shape) -> Honest {
    match s {
        Shape::MutResult
        | Shape::MutEnsures
        | Shape::MutOption
        | Shape::MutSelf
        | Shape::RenamedMut => Honest::Refused(MUT),
        Shape::MeasureOverride | Shape::MeasureSame | Shape::MeasureUnmarked => {
            Honest::Refused(FINAL)
        }
        Shape::MutUnnamed
        | Shape::MutPre
        | Shape::MutRefined
        | Shape::MutTarget
        | Shape::Renamed
        | Shape::MethodOverride
        | Shape::MeasureDefault => Honest::Accepted,
    }
}

/// How the trait is implemented.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Impl {
    /// For a struct of its own.
    Own,
    /// For a generic struct, at any `A`.
    Generic,
    /// For any type with an impl of another trait.
    Blanket,
}

/// The types and the impl's head, and a value of the impl's type.
fn impl_of(i: Impl) -> (&'static str, &'static str, &'static str) {
    match i {
        Impl::Own => (
            "#[derive(Discard)]\nstruct X { n: usize }\n",
            "impl Tr for X {\n",
            "X { n: 1 }",
        ),
        Impl::Generic => (
            "#[derive(Discard)]\nstruct X<A> { n: usize, a: A }\n",
            "impl<A: Discard> Tr for X<A> {\n",
            "X { n: 1, a: 0u8 }",
        ),
        Impl::Blanket => (
            "trait Q {\n    fn q(&self) -> usize;\n}\n\n#[derive(Discard)]\nstruct X { n: usize }\n\nimpl Q for X {\n    fn q(&self) -> usize { self.n }\n}\n",
            "impl<A: Q + Discard> Tr for A {\n",
            "X { n: 1 }",
        ),
    }
}

/// The program: `s`'s trait, implemented as `i`, keeping its word or not,
/// used generically or at the known type.
fn program(s: Shape, i: Impl, methods: &str, generic: bool) -> String {
    let (tr, _, _, call) = shape(s);
    let (types, head, make) = impl_of(i);
    let mut src = format!("trait Tr {{\n{tr}}}\n\n{types}\n{head}{methods}}}\n\n");
    if generic {
        let call = call.replace('T', "t");
        // `f` takes `self` by value, of a value whose measure is small.
        let (bound, param, arg) = match s {
            Shape::MutSelf => (
                "Tr + Discard",
                "t: { t: T | t.size() < 2 }",
                make.to_string(),
            ),
            _ => ("Tr", "t: &T", format!("&{make}")),
        };
        src.push_str(&format!(
            "fn g<T: {bound}>({param}) -> u8 {{\n    let a = [7u8; 2];\n    {call}\n}}\n\n\
             fn main() with {{}} {{\n    let _ = g({arg});\n}}\n"
        ));
    } else {
        let call = call.replace('T', "x");
        src.push_str(&format!(
            "fn main() with {{}} {{\n    let x = {make};\n    let a = [7u8; 2];\n    let _ = {call};\n}}\n"
        ));
    }
    src
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore)]
fn inheritance() {
    let shapes = [
        Shape::MutResult,
        Shape::MutEnsures,
        Shape::MutOption,
        Shape::MutUnnamed,
        Shape::MutPre,
        Shape::MutRefined,
        Shape::MutTarget,
        Shape::MutSelf,
        Shape::Renamed,
        Shape::RenamedMut,
        Shape::MethodOverride,
        Shape::MeasureOverride,
        Shape::MeasureSame,
        Shape::MeasureUnmarked,
        Shape::MeasureDefault,
    ];
    let mut failures = Vec::new();
    let mut count = 0;
    for s in shapes {
        for i in [Impl::Own, Impl::Generic, Impl::Blanket] {
            // A blanket impl can't make another `Self`, nor read a field.
            if s == Shape::MutSelf && i == Impl::Blanket {
                continue;
            }
            let (_, keeps, breaks, _) = shape(s);
            let bodies = [(true, Some(keeps)), (false, breaks)];
            for (honest_, methods) in bodies {
                let Some(methods) = methods else {
                    continue;
                };
                for generic in [true, false] {
                    count += 1;
                    let src = program(s, i, methods, generic);
                    let what = format!(
                        "{s:?} by a{} impl that {}, called {}",
                        match i {
                            Impl::Own => "n own",
                            Impl::Generic => " generic",
                            Impl::Blanket => " blanket",
                        },
                        if honest_ { "keeps its word" } else { "lies" },
                        if generic {
                            "generically"
                        } else {
                            "at its type"
                        },
                    );
                    match (verdict(&src), honest_, honest(s)) {
                        (Ok(()), true, Honest::Accepted) => {}
                        (Err(m), true, Honest::Refused(why)) if m.contains(why) => {}
                        // A lie is refused, by the rule against it if
                        // there's one: inheritance's, where its honest
                        // twin is refused by it too.
                        (Err(m), false, Honest::Refused(why)) if m.contains(why) => {}
                        (Err(m), false, Honest::Accepted) if !m.starts_with("accepted") => {}
                        (Ok(()), ..) => failures.push(format!("{what}: accepted\n{src}")),
                        (Err(m), ..) => failures.push(format!("{what}: {m}\n{src}")),
                    }
                }
            }
        }
    }
    assert!(count > 100, "only {count} programs");
    assert!(
        failures.is_empty(),
        "{} of {count} failed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}
