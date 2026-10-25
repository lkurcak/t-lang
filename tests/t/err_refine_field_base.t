// A field of a type parameter's type can be refined only by its measures,
// and a type parameter has none.
struct Wrap<T> { x: { v: T | v == v } }

fn main() {}
// error: a refinement can only name `v`, of type `T`, by its measures: `v.m()`, where `m` is a `#[measure]`
