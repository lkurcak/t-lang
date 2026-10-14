// `n` lives in `adder`'s stack frame, which is gone once it returns, so a
// closure that captures it can't be returned.
fn adder(n: int) -> fn(int) -> int { |x| x + n }
fn main() { print(adder(5)(1)); }
// error: cannot return a closure that captures `n`
