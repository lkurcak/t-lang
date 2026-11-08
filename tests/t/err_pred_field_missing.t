// A path's field must be the struct's.
#[derive(Copy, Discard)]
struct P { x: u8 }

fn f(p: P) -> { v: u8 | v == p.y } { p.x }

fn main() {}
// error: `P` has no field `y`
