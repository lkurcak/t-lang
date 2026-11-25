// A bound an impl's associated effect hangs on is decided of a type with no
// parameter in it, whichever way its impl is given: written, derived, or by
// the checker's rules, of ints, tuples, references and closures. Where it
// holds the atom is out; where it doesn't, in, and the copy says so
// (docs/96-associated-effects-unless.md, lemma A1).
use core::fmt::Debug;

trait Go {
    effect e;

    fn go(&self) -> u8 with Self::e;
}

#[derive(Discard where T: Discard)]
struct Wc<T> { x: T }

impl<T: Discard> Go for Wc<T> {
    effect e = panic unless T: Copy;

    fn go(&self) -> u8 with Self::e { 1 }
}

#[derive(Discard where T: Discard)]
struct Wo<T> { x: T }

impl<T: Discard> Go for Wo<T> {
    effect e = panic unless T: Ord;

    fn go(&self) -> u8 with Self::e { 2 }
}

#[derive(Discard where T: Discard)]
struct Wd<T> { x: T }

impl<T: Discard> Go for Wd<T> {
    effect e = panic unless T: Debug;

    fn go(&self) -> u8 with Self::e { 3 }
}

#[derive(Discard where T: Discard)]
struct We<T> { x: T }

impl<T: Discard> Go for We<T> {
    effect e = panic unless T: Eq;

    fn go(&self) -> u8 with Self::e { 4 }
}

#[derive(Copy, Discard, Eq, Ord, Debug)]
struct C { n: u8 }

#[derive(Discard)]
struct Plain { n: u8 }

/// Each bound holds: no `panic`.
fn all(c: C) -> (u8, u8, u8, u8) with {} {
    ((Wc { x: c }).go(), (Wo { x: c }).go(), (Wd { x: c }).go(), (We { x: c }).go())
}

/// By the checker's rules: ints, tuples, shared references, closures.
fn rules() -> (u8, u8, u8, u8) with {} {
    let k = 1u8;
    ((Wc { x: 7u8 }).go(), (We { x: (1u8, true) }).go(), (Wc { x: &k }).go(), (Wc { x: || k }).go())
}

/// None holds: each may panic, and says so.
fn none(p: Plain) -> (u8, u8, u8) with panic {
    let s = alloc::string::String::new();
    ((Wc { x: p }).go(), (Wo { x: Plain { n: 1 } }).go(), (Wc { x: move || s.len() }).go())
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let (a, b, c, d) = all(C { n: 1 });
    out.line(&(a as u32 * 1000 + b as u32 * 100 + c as u32 * 10 + d as u32))?; // expect: 1234
    let (a, b, c, d) = rules();
    out.line(&(a as u32 * 1000 + b as u32 * 100 + c as u32 * 10 + d as u32))?; // expect: 1411
    let (a, b, c) = none(Plain { n: 2 });
    out.line(&(a as u32 * 100 + b as u32 * 10 + c as u32))?; // expect: 121
    Ok(())
}
