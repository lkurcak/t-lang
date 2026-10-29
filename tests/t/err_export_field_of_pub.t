// A field is seen no further than its struct, so `export` on one of a
// struct that isn't exported says what isn't so.
#[derive(Discard)]
pub struct S { export x: i64 }

fn main() { let _s = S { x: 1 }; }
// error: field `x` can't be exported: `S` isn't
