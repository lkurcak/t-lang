fn first(o: Option<{ v: int | v > 0 }>) -> int { 1 }

fn main() {}
// error: a refinement can only be the whole type of a parameter, a result, a `let` or a struct's field
