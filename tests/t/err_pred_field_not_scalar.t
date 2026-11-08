// A field that isn't an integer or a `bool` is named by its fields and
// measures, not whole.
#[derive(Copy, Discard)]
struct P { x: u8 }

#[derive(Copy, Discard)]
struct L { from: P, to: P }

fn f(l: L, p: { p: P | p.x == 0 }) -> { b: bool | b == (l.from == l.to) } { true }

fn main() {}
// error: a refinement can only name `l.from`, of type `P`, by its fields and measures: `l.from.f`, or `l.from.m()`, where `m` is a `#[measure]`
