//! A generated family of programs whose closures capture fields
//! (docs/76-widths-and-captures.md): a closure reads, changes, copies or
//! moves `root.a.x` or `root.a.s`, where the root is a variable, what a
//! `&mut` points to or what a box holds; then, while it lives, the
//! function reads or changes a field next to it, the same field, a field
//! further off, or the struct it's in; then calls the closure.
//!
//! What the closure captures is worked out here from the rules, K1 to K4,
//! and with it whether what the function does next clashes with it. The
//! oracle is the interpreter's: each program that doesn't clash is
//! accepted and runs without "type soundness violated"; each that does is
//! refused.
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
    /// What a box holds, `b`.
    Box,
}

/// What the closure does.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Body {
    /// `|| root.a.x`, a copy: by `&`.
    Read,
    /// `|| root.a.x = 7`: by `&mut`.
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

const ROOTS: [Root; 3] = [Root::Var, Root::Ref, Root::Box];
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

fn captured(root: Root, body: Body) -> Captured {
    match (body, root) {
        // K1: by `&`, through references and boxes.
        (Body::Read, _) => Captured::Shared(&["a", "x"]),
        // K2: by `&mut`, fields of the variable only.
        (Body::Write, Root::Var) => Captured::Unique(&["a", "x"]),
        (Body::Write, _) => Captured::Unique(&[]),
        (Body::MoveRead, Root::Var) => Captured::Copied,
        // By value, not through a reference or box: the `&mut` or the box
        // is moved in.
        (Body::MoveRead, _) => Captured::Moved(&[]),
        (Body::Move, Root::Var) => Captured::Moved(&["a", "s"]),
        (Body::Move, _) => Captured::Moved(&[]),
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
fn clashes(root: Root, body: Body, then: Then) -> bool {
    // Moving out of what a `&mut` points to is refused anyway.
    if body == Body::Move && root == Root::Ref {
        return true;
    }
    let Some((place, writes)) = used(then) else {
        return false;
    };
    match captured(root, body) {
        Captured::Shared(c) => writes && overlap(c, place),
        Captured::Unique(c) => overlap(c, place),
        Captured::Copied => false,
        Captured::Moved(c) => overlap(c, place),
    }
}

const ITEMS: &str = "\
use alloc::boxed::Box;
use alloc::string::String;

#[derive(Discard)]
struct In { x: u64, y: u64, s: String }

#[derive(Discard)]
struct P { a: In, z: u64 }

fn take(s: String) -> u64 { s.len() as u64 }

fn make() -> P with alloc { P { a: In { x: 1, y: 2, s: String::from(\"ab\") }, z: 3 } }

";

fn program(root: Root, body: Body, then: Then) -> String {
    let r = match root {
        Root::Var => "p",
        Root::Ref => "r",
        Root::Box => "b",
    };
    let closure = match body {
        Body::Read => format!("let c = || {r}.a.x;"),
        Body::Write => format!("let mut c = || {{ {r}.a.x = 7; 0u64 }};"),
        Body::MoveRead => format!("let c = move || {r}.a.x;"),
        Body::Move => format!("let c = move || take({r}.a.s);"),
    };
    let then = match then {
        Then::Nothing => String::new(),
        Then::ReadNext => format!("let _v = {r}.a.y;"),
        Then::WriteNext => format!("{r}.a.y = 5;"),
        Then::ReadSame => format!("let _v = {r}.a.x;"),
        Then::WriteSame => format!("{r}.a.x = 5;"),
        Then::ReadFar => format!("let _v = {r}.z;"),
        Then::WriteFar => format!("{r}.z = 5;"),
        Then::BorrowParent => format!("let _q = &{r}.a;"),
    };
    let code = format!("{closure}\n    {then}\n    let _w = c();");
    match root {
        Root::Var => format!(
            "{ITEMS}#[allow(unused_mut)]\nfn main() with {{alloc, panic}} {{\n    \
             let mut p = make();\n    {code}\n}}\n"
        ),
        Root::Ref => format!(
            "{ITEMS}fn f(r: &mut P) with panic {{\n    {code}\n}}\n\n\
             fn main() with {{alloc, panic}} {{\n    let mut p = make();\n    f(&mut p);\n}}\n"
        ),
        Root::Box => format!(
            "{ITEMS}#[allow(unused_mut)]\nfn main() with {{alloc, panic}} {{\n    \
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
        for body in BODIES {
            for then in THENS {
                let src = program(root, body, then);
                let label = format!("{root:?} × {body:?} × {then:?}");
                match (clashes(root, body, then), verdict(&src)) {
                    (_, Verdict::Broke(why)) => off.push(format!("{label}: broke: {why}\n{src}")),
                    (false, Verdict::Ran) | (true, Verdict::Refused(_)) => {}
                    (_, Verdict::Ran) => off.push(format!("{label}: accepted\n{src}")),
                    (_, Verdict::Refused(e)) => off.push(format!("{label}: refused: {e}\n{src}")),
                }
            }
        }
    }
    assert!(off.is_empty(), "{} off:\n\n{}", off.len(), off.join("\n\n"));
}
