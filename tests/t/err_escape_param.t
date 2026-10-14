// A parameter may be a closure that captures the caller's variables. If it
// could be returned, `adder` below could return `|x| x + n` through it.
fn id(f: fn(int) -> int) -> fn(int) -> int { f }
fn adder(n: int) -> fn(int) -> int { id(|x| x + n) }
fn main() { print(adder(5)(1)); }
// error: cannot return `f`: it may be a closure that captures
