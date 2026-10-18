// The same for a field of a function type: a struct can be returned, and
// the closure with it.
#[derive(Discard)]
struct F { f: fn(int) -> int }
fn adder(n: int) -> F { F { f: |x| x + n } }
fn main() { print((adder(5).f)(1)); }
// error: expected `fn(int) -> int`, found a closure that captures `n`
