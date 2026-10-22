fn pick(x: &i64, y: &i64) -> &i64 { x }

fn main() {}
// error: missing lifetime: this returns a reference, but the parameters have 2 lifetimes
