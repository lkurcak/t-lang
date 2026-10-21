// Only `int` and `bool` fields can be refined.
struct Wrap<T> { x: { v: T | v == v } }

fn main() {}
// error: only `int` and `bool` can be refined, not `T`
