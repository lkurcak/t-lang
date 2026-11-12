//! A generated family of programs whose `extern fn` exports do something
//! C may not want done while it's interrupted (docs/71-t-metal-findings.md,
//! docs/40-extern.md). Each is the product of
//!
//! - a **function**: an export C may call from an interrupt, or the
//!   platform's `t_platform_write`, which the runtime calls while it
//!   panics;
//! - an **effect** its body has: `alloc`, `panic`, `global` or none;
//! - a **way** its body gets it: doing it, saying so with `with` too, or
//!   through a function that says so, a private function whose effects are
//!   inferred, a closure, a generic function given a closure, a method, or
//!   a trait's method with an associated effect.
//!
//! The oracle is the rule: an export may have only `global` and `panic`,
//! as allocating could start the allocator again while the code it
//! interrupted is in it; the platform's function only `global`. Each
//! program the rule refuses must be refused for having that effect, and
//! each it allows must check. Checking is the same on both targets, so
//! each program must be refused or not the same way for both.
//!
//! They're about what programs mean, the same on every platform, so they run
//! on Linux alone.

use std::path::Path;
use t_lang::Target;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Function {
    Export,
    Platform,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Effect {
    Alloc,
    Panic,
    Global,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Way {
    /// The body does it.
    Itself,
    /// The body does it, and says so with `with`.
    Written,
    /// Through a function that says so.
    Callee,
    /// Through a private function whose effects are inferred.
    Inferred,
    /// Through a closure the body calls.
    Closure,
    /// Through a generic function given a closure that does it.
    Generic,
    /// Through a method, whose effects are inferred.
    Method,
    /// Through a trait's method, whose associated effect says it.
    Trait,
}

const FUNCTIONS: [Function; 2] = [Function::Export, Function::Platform];
const EFFECTS: [Effect; 4] = [Effect::Alloc, Effect::Panic, Effect::Global, Effect::None];
const WAYS: [Way; 8] = [
    Way::Itself,
    Way::Written,
    Way::Callee,
    Way::Inferred,
    Way::Closure,
    Way::Generic,
    Way::Method,
    Way::Trait,
];

const ITEMS: &str = "use alloc::boxed::Box;
use core::sync::Atomic;

static LAST: Atomic<u8> = Atomic::new(0);

#[derive(Discard)]
struct Device {}

";

impl Function {
    fn name(self) -> &'static str {
        match self {
            Function::Export => "on_interrupt",
            Function::Platform => "t_platform_write",
        }
    }

    /// May it have `e`?
    fn may(self, e: Effect) -> bool {
        match self {
            Function::Export => e != Effect::Alloc,
            Function::Platform => matches!(e, Effect::Global | Effect::None),
        }
    }
}

impl Effect {
    /// What a `with` says to have it.
    fn set(self) -> &'static str {
        match self {
            Effect::Alloc => "alloc",
            Effect::Panic => "panic",
            Effect::Global => "global",
            Effect::None => "{}",
        }
    }

    /// A statement that has it, of `b: u8`.
    fn statement(self) -> &'static str {
        match self {
            Effect::Alloc => "let _ = Box::new(b);",
            Effect::Panic => "if b == 0 { panic(\"zero\") }",
            Effect::Global => "LAST.store(b);",
            Effect::None => "let _ = b;",
        }
    }

    /// What the error says when it can't be had.
    fn refused(self) -> String {
        let name = match self {
            Effect::Alloc => "alloc",
            Effect::Panic => "panic",
            Effect::Global => "global",
            Effect::None => unreachable!("nothing is always allowed"),
        };
        format!("can't have `{name}`")
    }
}

/// The program where `f` gets `e` the way `w` says.
fn program(f: Function, e: Effect, w: Way) -> String {
    let (name, set, s) = (f.name(), e.set(), e.statement());
    let (items, body) = match w {
        Way::Itself => (String::new(), s.to_string()),
        Way::Written => {
            return format!(
                "{ITEMS}extern fn {name}(b: u8) with {set} {{\n    {s}\n}}\n\nfn main() {{}}\n"
            );
        }
        Way::Callee => (
            format!("fn act(b: u8) with {set} {{\n    {s}\n}}\n\n"),
            "act(b)".to_string(),
        ),
        Way::Inferred => (
            format!("fn act(b: u8) {{\n    {s}\n}}\n\n"),
            "act(b)".to_string(),
        ),
        Way::Closure => (
            String::new(),
            format!("let f = |b: u8| {{ {s} }};\n    f(b)"),
        ),
        Way::Generic => (
            "fn apply<effect g, F: FnOnce(u8) with g>(f: F, b: u8) with g { f(b) }\n\n".to_string(),
            format!("apply(|b: u8| {{ {s} }}, b)"),
        ),
        Way::Method => (
            format!("impl Device {{\n    fn act(&self, b: u8) {{\n        {s}\n    }}\n}}\n\n"),
            "let d = Device {};\n    d.act(b)".to_string(),
        ),
        Way::Trait => (
            format!(
                "trait Act {{\n    effect e;\n\n    fn act(&self, b: u8) with Self::e;\n}}\n\n\
                 impl Act for Device {{\n    effect e = {set};\n\n    \
                 fn act(&self, b: u8) with {set} {{\n        {s}\n    }}\n}}\n\n\
                 fn via<A: Act>(a: &A, b: u8) with A::e {{ a.act(b) }}\n\n"
            ),
            "let d = Device {};\n    via(&d, b)".to_string(),
        ),
    };
    format!("{ITEMS}{items}extern fn {name}(b: u8) {{\n    {body}\n}}\n\nfn main() {{}}\n")
}

/// What checking `src` for `target` says: nothing, or the error.
fn checked(src: &str, target: Target) -> Option<String> {
    let checked = t_lang::check_files_for(Path::new("main.t"), src, &t_lang::no_files, target);
    (checked.result.err()).map(|e| e.render_in(&checked.sources))
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore = "the same on every platform")]
fn exports_keep_to_what_an_interrupt_allows() {
    let mut failures = Vec::new();
    let mut n = 0;
    for f in FUNCTIONS {
        for e in EFFECTS {
            for w in WAYS {
                n += 1;
                let src = program(f, e, w);
                let hosted = checked(&src, Target::Hosted);
                let freestanding = checked(&src, Target::Freestanding);
                let what = format!("{f:?} × {e:?} × {w:?}");
                if hosted != freestanding {
                    failures.push(format!(
                        "{what}: checked differently on the two targets:\n\
                         hosted: {hosted:?}\nfreestanding: {freestanding:?}\n\n{src}"
                    ));
                    continue;
                }
                match (f.may(e), hosted) {
                    (true, None) => {}
                    (true, Some(why)) => failures.push(format!("{what}: refused\n{why}\n{src}")),
                    (false, None) => failures.push(format!("{what}: accepted\n{src}")),
                    (false, Some(why)) if why.contains(&e.refused()) => {}
                    (false, Some(why)) => failures.push(format!(
                        "{what}: refused, but not for `{}`\n{why}\n{src}",
                        e.refused()
                    )),
                }
            }
        }
    }
    assert_eq!(n, 64);
    assert!(
        failures.is_empty(),
        "{} of {n} went wrong:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}
