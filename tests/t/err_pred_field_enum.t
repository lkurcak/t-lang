// An enum has no fields: a predicate says what's in its variants by a
// refinement of its type argument.
fn f(o: Option<u8>) -> { v: u8 | v == o.x } { 0 }

fn main() {}
// error: `Option<u8>` has no field `x`
