fn inc(x: i64) -> i64 with panic { x + 1 }
fn main() { print(inc::<i64>(1)); }
// error: `inc` takes no type arguments
