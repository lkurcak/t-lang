// The same for a field of a function type: a struct can be returned, and
// the closure with it.
#[derive(Discard)]
struct F { f: fn(i64) -> i64 }
fn adder(n: i64) -> F { F { f: |x| x + n } }
fn main() { print((adder(5).f)(1)); }
// error: expected `fn(i64) -> i64`, found a closure that captures `n`
