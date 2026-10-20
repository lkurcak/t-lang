fn forever(n: int) -> int with panic { forever(n + 1) }
fn main() -> int with panic { forever(0) }
// error: runtime error: stack overflow
