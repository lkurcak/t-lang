fn apply(f: fn(int) -> int, x: int) -> int { f(x) }
fn is_zero(x: int) -> bool { x == 0 }
fn main() -> int { apply(is_zero, 1) }
// error: expected `fn(int) -> int`, found `fn(int) -> bool`
