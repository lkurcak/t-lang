// An `i64` may be too big to be an `f64` exactly: 2^53 + 1 is between two
// of them. Nothing here says it isn't.
fn f(n: i64) -> f64 with {} { n as f64 }
// error: `as` may panic, but `f` doesn't say `with panic`

fn main() {}
