fn f(x: int) -> int { x }
fn main() -> bool { f == f }
// error: cannot compare functions with `==`
