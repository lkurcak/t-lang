// `nest::<int>` would need `nest::<Option<int>>`, which would need
// `nest::<Option<Option<int>>>`, and so on: monomorphising never ends.
#[derive(Discard)]
enum Option<T> { Some(T), None }
fn nest<T: Discard>(x: T, n: int) -> int {
    if n == 0 { 0 } else { nest(Option::Some(x), n - 1) }
}
fn main() { print(nest(1, 3)); }
// error: polymorphic recursion: `nest` uses itself with `T` = `Option<T>`
