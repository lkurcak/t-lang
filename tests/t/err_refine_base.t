fn f(o: { v: Option<i64> | true }) -> i64 { 1 }

fn main() {}
// error: only integer types and `bool` can be refined, not `Option<i64>`
