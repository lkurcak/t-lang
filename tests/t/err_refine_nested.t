fn first(o: Option<{ v: i64 | v > 0 }>) -> i64 { 1 }

fn main() {}
// error: a refinement can only be the whole type of a parameter, a result, a `let` or a struct's field
