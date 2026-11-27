//! A generated family of programs whose closures capture fields
//! (docs/76-widths-and-captures.md, docs/77-method-captures.md): a closure
//! reads, changes, copies or moves `root.a.x` or `root.a.s`, where the root
//! is a variable, what a `&mut` parameter points to (`self` too, or one
//! declared `mut`), what a reference in a variable points to, or what a box
//! holds; and the structs' refinements relate `x` to nothing, to `y`, to
//! itself alone, or `a` to `z`, or `a` has one of its own. Then, while the
//! closure lives, the function reads or changes a field next to it, the
//! same field, a field further off, or the struct it's in; then calls the
//! closure.
//!
//! What the closure captures is worked out here from the rules, K1 to K4,
//! and with it whether what the function does next clashes with it. The
//! oracle is the interpreter's: each program that doesn't clash is
//! accepted and runs without "type soundness violated"; each that does is
//! refused. What's written keeps every refinement (`x` only ever becomes
//! 0, `y` and `z` the largest `u64`), so a refusal is a clash.
//!
//! They're about what programs mean, the same on every platform, so they
//! run on Linux alone.

use std::panic::catch_unwind;

/// Where the struct is.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Root {
    /// A variable, `p`.
    Var,
    /// What a `&mut` parameter points to, `r`.
    Ref,
    /// What a method's `&mut self` points to.
    SelfRef,
    /// What a `&mut` parameter declared `mut` points to.
    MutRef,
    /// What a `&mut` in a variable points to, `let r = &mut p`.
    LocalRef,
    /// What a box holds, `b`.
    Box,
}

/// What the structs' refinements relate.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Shape {
    /// Nothing.
    Plain,
    /// `y` to `x`: `In { x, y: { v | v >= x }, s }`.
    Related,
    /// `x` to itself alone: `x: { v | v < 100 }`.
    Alone,
    /// The places in `a`: `P { a: { a: In | a.x <= a.y }, z }`.
    Outer,
    /// `z` to `a`: `P { a, z: { v | v >= a.x } }`.
    Far,
}

/// What the closure does.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Body {
    /// `|| root.a.x`, a copy: by `&`.
    Read,
    /// `|| root.a.x = 0`: by `&mut`.
    Write,
    /// `move || root.a.x`: by value.
    MoveRead,
    /// `move || take(root.a.s)`, moving a `String` out.
    Move,
}

/// What the function does while the closure lives.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Then {
    Nothing,
    ReadNext,
    WriteNext,
    ReadSame,
    WriteSame,
    ReadFar,
    WriteFar,
    /// `&root.a`, all of the struct the field is in.
    BorrowParent,
}

const ROOTS: [Root; 6] = [
    Root::Var,
    Root::Ref,
    Root::SelfRef,
    Root::MutRef,
    Root::LocalRef,
    Root::Box,
];
const SHAPES: [Shape; 5] = [
    Shape::Plain,
    Shape::Related,
    Shape::Alone,
    Shape::Outer,
    Shape::Far,
];
const BODIES: [Body; 4] = [Body::Read, Body::Write, Body::MoveRead, Body::Move];
const THENS: [Then; 8] = [
    Then::Nothing,
    Then::ReadNext,
    Then::WriteNext,
    Then::ReadSame,
    Then::WriteSame,
    Then::ReadFar,
    Then::WriteFar,
    Then::BorrowParent,
];

/// A place in the root: its fields from it.
type Path = &'static [&'static str];

/// What the closure captures, by the rules: the place, and how.
#[derive(Debug, PartialEq)]
enum Captured {
    /// By `&`, `&mut` (or uniquely), copied, or moved.
    Shared(Path),
    Unique(Path),
    Copied,
    Moved(Path),
}

/// How far into the root a closure changing or taking a place in `a`
/// goes, by K2: past `a` if no refinement relates it to `z` and it has
/// none of its own; then past the field in `a` if no refinement relates
/// it to another.
fn apart(shape: Shape, field: &'static str) -> Path {
    match (shape, field) {
        (Shape::Far, _) => &[],
        (Shape::Outer, _) => &["a"],
        (Shape::Related, "x") => &["a"],
        (_, "x") => &["a", "x"],
        _ => &["a", "s"],
    }
}

fn captured(root: Root, shape: Shape, body: Body) -> Captured {
    // K2: by `&mut`, through a parameter's `&mut` only; by value, through
    // none.
    let through = matches!(root, Root::Var | Root::Ref | Root::SelfRef);
    match body {
        // K1: by `&`, through references and boxes.
        Body::Read => Captured::Shared(&["a", "x"]),
        Body::Write if through => Captured::Unique(apart(shape, "x")),
        Body::Write => Captured::Unique(&[]),
        // By value, not through a reference or box: the `&mut` or the box
        // is moved in.
        _ if root != Root::Var => Captured::Moved(&[]),
        // A copy of `x`, if it goes that far; else what it stops at,
        // moved in, as `In` isn't `Copy`.
        Body::MoveRead => match apart(shape, "x") {
            [_, _] => Captured::Copied,
            p => Captured::Moved(p),
        },
        Body::Move => Captured::Moved(apart(shape, "s")),
    }
}

/// The place `then` uses, and whether it changes it.
fn used(then: Then) -> Option<(Path, bool)> {
    match then {
        Then::Nothing => None,
        Then::ReadNext => Some((&["a", "y"], false)),
        Then::WriteNext => Some((&["a", "y"], true)),
        Then::ReadSame => Some((&["a", "x"], false)),
        Then::WriteSame => Some((&["a", "x"], true)),
        Then::ReadFar => Some((&["z"], false)),
        Then::WriteFar => Some((&["z"], true)),
        Then::BorrowParent => Some((&["a"], false)),
    }
}

fn overlap(a: Path, b: Path) -> bool {
    a.starts_with(b) || b.starts_with(a)
}

/// Does what the function does clash with what the closure holds?
fn clashes(root: Root, shape: Shape, body: Body, then: Then) -> bool {
    // Moving out of what a `&mut` points to is refused anyway.
    if body == Body::Move && !matches!(root, Root::Var | Root::Box) {
        return true;
    }
    let Some((place, writes)) = used(then) else {
        return false;
    };
    match captured(root, shape, body) {
        Captured::Shared(c) => writes && overlap(c, place),
        Captured::Unique(c) => overlap(c, place),
        Captured::Copied => false,
        Captured::Moved(c) => overlap(c, place),
    }
}

const COMMON: &str = "\
use alloc::boxed::Box;
use alloc::string::String;

fn take(s: String) -> u64 { s.len() as u64 }

fn make() -> P with alloc { P { a: In { x: 1, y: 20, s: String::from(\"ab\") }, z: 30 } }

";

fn structs(shape: Shape) -> String {
    let (x, y, a, z) = match shape {
        Shape::Plain => ("u64", "u64", "In", "u64"),
        Shape::Related => ("u64", "{ v: u64 | v >= x }", "In", "u64"),
        Shape::Alone => ("{ v: u64 | v < 100 }", "u64", "In", "u64"),
        Shape::Outer => ("u64", "u64", "{ a: In | a.x <= a.y }", "u64"),
        Shape::Far => ("u64", "u64", "In", "{ v: u64 | v >= a.x }"),
    };
    format!(
        "#[derive(Discard)]\nstruct In {{ x: {x}, y: {y}, s: String }}\n\n\
         #[derive(Discard)]\nstruct P {{ a: {a}, z: {z} }}\n\n{COMMON}"
    )
}

fn program(root: Root, shape: Shape, body: Body, then: Then) -> String {
    let r = match root {
        Root::Var => "p",
        Root::Ref | Root::MutRef | Root::LocalRef => "r",
        Root::SelfRef => "self",
        Root::Box => "b",
    };
    let max = "18446744073709551615";
    let closure = match body {
        Body::Read => format!("let c = || {r}.a.x;"),
        Body::Write => format!("let mut c = || {{ {r}.a.x = 0; 0u64 }};"),
        Body::MoveRead => format!("let c = move || {r}.a.x;"),
        Body::Move => format!("let c = move || take({r}.a.s);"),
    };
    let then = match then {
        Then::Nothing => String::new(),
        Then::ReadNext => format!("let _v = {r}.a.y;"),
        Then::WriteNext => format!("{r}.a.y = {max};"),
        Then::ReadSame => format!("let _v = {r}.a.x;"),
        Then::WriteSame => format!("{r}.a.x = 0;"),
        Then::ReadFar => format!("let _v = {r}.z;"),
        Then::WriteFar => format!("{r}.z = {max};"),
        Then::BorrowParent => format!("let _q = &{r}.a;"),
    };
    let code = format!("{closure}\n    {then}\n    let _w = c();");
    let items = structs(shape);
    let main = "#[allow(unused_mut)]\nfn main() with {alloc, panic} {\n    let mut p = make();\n";
    match root {
        Root::Var => format!("{items}{main}    {code}\n}}\n"),
        Root::Ref => format!(
            "{items}fn f(r: &mut P) with panic {{\n    {code}\n}}\n\n{main}    f(&mut p);\n}}\n"
        ),
        Root::SelfRef => format!(
            "{items}impl P {{\n    fn f(&mut self) with panic {{\n    {code}\n    }}\n}}\n\n\
             {main}    p.f();\n}}\n"
        ),
        Root::MutRef => format!(
            "{items}#[allow(unused_mut)]\nfn f(mut r: &mut P) with panic {{\n    {code}\n}}\n\n\
             {main}    f(&mut p);\n}}\n"
        ),
        Root::LocalRef => format!("{items}{main}    let r = &mut p;\n    {code}\n}}\n"),
        Root::Box => format!(
            "{items}#[allow(unused_mut)]\nfn main() with {{alloc, panic}} {{\n    \
             let mut b = Box::new(make());\n    {code}\n}}\n"
        ),
    }
}

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

/// Each program whose function doesn't touch what its closure holds is
/// accepted and runs; each other is refused; none breaks a type at run
/// time.
#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn closures_hold_only_what_they_capture() {
    let mut off = Vec::new();
    for root in ROOTS {
        for shape in SHAPES {
            for body in BODIES {
                for then in THENS {
                    let src = program(root, shape, body, then);
                    let label = format!("{root:?} × {shape:?} × {body:?} × {then:?}");
                    match (clashes(root, shape, body, then), verdict(&src)) {
                        (_, Verdict::Broke(why)) => {
                            off.push(format!("{label}: broke: {why}\n{src}"))
                        }
                        (false, Verdict::Ran) | (true, Verdict::Refused(_)) => {}
                        (_, Verdict::Ran) => off.push(format!("{label}: accepted\n{src}")),
                        (_, Verdict::Refused(e)) => {
                            off.push(format!("{label}: refused: {e}\n{src}"))
                        }
                    }
                }
            }
        }
    }
    assert!(off.is_empty(), "{} off:\n\n{}", off.len(), off.join("\n\n"));
}
