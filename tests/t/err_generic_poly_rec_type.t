// `Tree<i64>` would need `Tree<Pair<i64>>`, and so on.
#[derive(Discard)]
struct Pair<T> { a: T, b: T }
#[derive(Discard)]
struct Tree<T> { leaf: T, deeper: fn(i64) -> Tree<Pair<T>> }
fn main() {}
// error: polymorphic recursion: `Tree` uses itself with `T` = `Pair<T>`
