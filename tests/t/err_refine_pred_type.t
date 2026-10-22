fn f(k: { v: i64 | v + 1 }) -> i64 { k }

fn main() {}
// error: expected `bool`, found an integer
