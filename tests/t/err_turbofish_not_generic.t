fn inc(x: int) -> int { x + 1 }
fn main() { print(inc::<int>(1)); }
// error: `inc` takes no type arguments
