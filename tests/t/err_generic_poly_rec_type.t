// `Tree<int>` would need `Tree<Pair<int>>`, and so on.
struct Pair<T> { a: T, b: T }
struct Tree<T> { leaf: T, deeper: fn(int) -> Tree<Pair<T>> }
fn main() {}
// error: polymorphic recursion: `Tree` uses itself with `T` = `Pair<T>`
