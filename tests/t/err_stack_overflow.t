fn forever(n: int) -> int { forever(n + 1) }
fn main() -> int { forever(0) }
// error: runtime error: stack overflow
