fn add(a: int, b: int) -> int with panic { a + b }
fn main() -> int { add(1) }
// error: this function takes 2 arguments but 1 was supplied
