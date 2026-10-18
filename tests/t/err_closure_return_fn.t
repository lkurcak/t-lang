// A function type holds functions, and closures that capture nothing: a
// closure that captures carries what it captures, so it isn't one. `n`
// lives in `adder`'s stack frame, which is gone once it returns.
fn adder(n: int) -> fn(int) -> int { |x| x + n }
fn main() { print(adder(5)(1)); }
// error: expected `fn(int) -> int`, found a closure that captures `n`
