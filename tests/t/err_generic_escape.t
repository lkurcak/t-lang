// `id` returns its argument. Its copy for a function type would return a
// closure that captures `n`, which lives on `make`'s stack.
fn id<T>(x: T) -> T { x }
fn make(n: int) -> fn(int) -> int { id(|y| y + n) }
fn main() { print(make(1)(2)); }
// error: cannot use `id` with `T` = `fn(int) -> int` here: in that copy of it, cannot return `x`
