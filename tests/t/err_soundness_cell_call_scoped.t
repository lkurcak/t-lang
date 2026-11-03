// A call-scoped reference can't escape into a cell the caller holds
// (lemma P1 of docs/30-postconditions.md), as the cell is invariant.
use core::cell::Cell;

fn stash<'a>(c: &Cell<&'a i64>, x: &i64) { c.set(x); }

fn main() {}
// error: lifetime may not live long enough
