fn f(n: { n: i64 | n < 1.5 }) -> i64 { n }
// error: a refinement can't have a float in it: floats aren't in its logic

fn main() {}
