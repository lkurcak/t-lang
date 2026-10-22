// An enum's variants can't have refinements yet.
enum E { A({ v: i64 | v > 0 }) }

fn main() {}
// error: a refinement can only be the whole type of a parameter, a result, a `let` or a struct's field
