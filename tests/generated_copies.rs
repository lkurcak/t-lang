//! A generated family of programs that copy refined values into a place
//! of refined values (docs/75-waiting-and-copying.md): each kind of copy
//! (`copy_from_slice` into an array, part of a list, a struct's field, or
//! through a generic function of our own; `List::extend_from_slice`;
//! `List::from_slice`) of values declared as the place's are, more
//! strongly, more weakly, or not at all, holding a value that keeps the
//! place's refinement or breaks it.
//!
//! The oracle is the interpreter's: every program is refused, or runs
//! without "type soundness violated", reading each value copied as one of
//! the place's. Each program whose values are declared at least as
//! strongly as the place's is accepted and runs; each other is refused,
//! for the refinement.
//!
//! They're about what programs mean, the same on every platform, so they run
//! on Linux alone.

use std::panic::catch_unwind;

/// What the values copied are declared as.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Source {
    /// `{ v: u8 | v < 10 }`, as the place's.
    Same,
    /// `{ v: u8 | v < 5 }`, which says more.
    Stronger,
    /// `{ v: u8 | v < 20 }`, which says less.
    Weaker,
    /// `u8`.
    Plain,
}

impl Source {
    fn ty(self) -> &'static str {
        match self {
            Source::Same => "{ v: u8 | v < 10 }",
            Source::Stronger => "{ v: u8 | v < 5 }",
            Source::Weaker => "{ v: u8 | v < 20 }",
            Source::Plain => "u8",
        }
    }

    /// Does what it says of its values say what the place's does?
    fn keeps(self) -> bool {
        matches!(self, Source::Same | Source::Stronger)
    }
}

/// How they're copied, and where to.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Copy {
    /// Into an array, `copy_from_slice`.
    Array,
    /// Into part of a list, `l.slice_mut(1..3).copy_from_slice(..)`.
    Part,
    /// Into a struct's field, an array.
    Field,
    /// Through `fill`, a generic function of our own over `copy_from_slice`.
    Generic,
    /// `List::extend_from_slice`, onto an empty list.
    Extend,
    /// `List::from_slice`.
    FromSlice,
}

const SOURCES: [Source; 4] = [
    Source::Same,
    Source::Stronger,
    Source::Weaker,
    Source::Plain,
];

const COPIES: [Copy; 6] = [
    Copy::Array,
    Copy::Part,
    Copy::Field,
    Copy::Generic,
    Copy::Extend,
    Copy::FromSlice,
];

const ITEMS: &str = "\
use alloc::list::List;

#[derive(Discard)]
struct D { cells: [{ v: u8 | v < 10 }; 2] }

fn fill<T: Copy>(d: &mut [T], s: { s: &[T] | s.len() == d.len() }) { d.copy_from_slice(s) }

fn need(v: { v: u8 | v < 10 }) -> u8 { v }

";

/// A program copying two `n`s, declared as `source` says, the way `copy`
/// does, into places of digits, and reading them as digits.
fn program(source: Source, copy: Copy, n: u8) -> String {
    let src = format!("let src: [{}; 2] = [{n}, {n}];", source.ty());
    let (code, read) = match copy {
        Copy::Array => (
            "let mut dst: [{ v: u8 | v < 10 }; 2] = [0, 0];\n    \
             dst.as_mut_slice().copy_from_slice(src.as_slice());",
            "(dst[0], dst[1])",
        ),
        Copy::Part => (
            "let mut dst: List<{ v: u8 | v < 10 }> = List::filled(4, 0);\n    \
             dst.slice_mut(1..3).copy_from_slice(src.as_slice());",
            "(dst[1], dst[2])",
        ),
        Copy::Field => (
            "let mut dst = D { cells: [0, 0] };\n    \
             dst.cells.as_mut_slice().copy_from_slice(src.as_slice());",
            "(dst.cells[0], dst.cells[1])",
        ),
        Copy::Generic => (
            "let mut dst: [{ v: u8 | v < 10 }; 2] = [0, 0];\n    \
             fill(dst.as_mut_slice(), src.as_slice());",
            "(dst[0], dst[1])",
        ),
        Copy::Extend => (
            "let mut dst: List<{ v: u8 | v < 10 }> = List::new();\n    \
             dst.extend_from_slice(src.as_slice());",
            "(dst[0], dst[1])",
        ),
        Copy::FromSlice => (
            "let dst: List<{ v: u8 | v < 10 }> = List::from_slice(src.as_slice());",
            "(dst[0], dst[1])",
        ),
    };
    format!(
        "{ITEMS}fn main() with alloc {{\n    {src}\n    {code}\n    let (a, b) = {read};\n    \
         let _ = (need(a), need(b));\n}}\n"
    )
}

#[derive(Debug)]
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

/// Each program copying values declared at least as the place's are is
/// accepted and runs; each other is refused, for the refinement, whatever
/// the values are; none breaks a refinement at run time.
#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn copies_keep_what_is_declared() {
    let mut off = Vec::new();
    for source in SOURCES {
        for copy in COPIES {
            // 3 is a digit; 15 isn't, but only a weaker declaration has it.
            let values: &[u8] = if source.keeps() { &[3] } else { &[3, 15] };
            for &n in values {
                let src = program(source, copy, n);
                let label = format!("{source:?} × {copy:?}, {n}");
                match (source.keeps(), verdict(&src)) {
                    (_, Verdict::Broke(why)) => off.push(format!("{label}: broke: {why}\n{src}")),
                    (true, Verdict::Ran) => {}
                    (false, Verdict::Refused(e)) if e.contains("can't prove `v < 10`") => {}
                    (_, got) => off.push(format!("{label}: {got:?}\n{src}")),
                }
            }
        }
    }
    assert!(off.is_empty(), "{} off:\n\n{}", off.len(), off.join("\n\n"));
}
