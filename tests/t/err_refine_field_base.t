// Only `i64` and `bool` fields can be refined.
struct Wrap<T> { x: { v: T | v == v } }

fn main() {}
// error: only integer types and `bool` can be refined, not `T`
