fn add(a: i64, b: i64) -> i64 with panic { a + b }
fn main() -> i64 { add(1) }
// error: this function takes 2 arguments but 1 was supplied
