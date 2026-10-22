fn forever(n: i64) -> i64 with panic { forever(n + 1) }
fn main() -> i64 with panic { forever(0) }
// error: runtime error: stack overflow
