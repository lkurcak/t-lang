// `Tree<int>` would need `Tree<Pair<int>>`, and so on.
#[derive(Discard)]
struct Pair<T> { a: T, b: T }
#[derive(Discard)]
struct Tree<T> { leaf: T, deeper: fn(int) -> Tree<Pair<T>> }
fn main() {}
// error: polymorphic recursion: `Tree` uses itself with `T` = `Pair<T>`
