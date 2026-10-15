fn pick(x: &int, y: &int) -> &int { x }

fn main() {}
// error: missing lifetime: this returns a reference, but the parameters have 2 lifetimes
