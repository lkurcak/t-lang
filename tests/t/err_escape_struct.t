// Storing a closure that captures is as good as returning it: the struct
// can be returned.
#[derive(Discard)]
struct F { f: fn(int) -> int }
fn adder(n: int) -> F { F { f: |x| x + n } }
fn main() { print((adder(5).f)(1)); }
// error: cannot store a closure that captures `n` in a struct or enum
