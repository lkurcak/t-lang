fn f(o: { v: Option<int> | true }) -> int { 1 }

fn main() {}
// error: only `int` and `bool` can be refined, not `Option<int>`
