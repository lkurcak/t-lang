fn f(x: f64) -> char { x as char }
// error: `as` converts between integer and floating-point types and `char`, not to `char`

fn main() {}
