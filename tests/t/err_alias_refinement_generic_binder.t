// An alias's parameters have no bounds, so its predicate can't take a `T`
// for an integer: it's checked where it's declared, for every `T`.
type Pos<T> = { v: T | v > 0 };

fn main() {
    let _x: Pos<i64> = 1;
}
// error: a refinement can only name `v`, of type `T`, by its measures
