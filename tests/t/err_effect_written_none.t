// `with {}` says there are none, so the body is checked against it.
fn div(n: i64, d: i64) -> i64 with {} { n / d }

fn main() { let _ = div(4, 2); }
// error: `/` may panic, but `div` doesn't say `with panic`
