// `nest::<i64>` would need `nest::<Option<i64>>`, which would need
// `nest::<Option<Option<i64>>>`, and so on: monomorphising never ends.
#[derive(Discard)]
enum Option<T> { Some(T), None }
fn nest<T: Discard>(x: T, n: i64) -> i64 with panic {
    if n == 0 { 0 } else { nest(Option::Some(x), n - 1) }
}
fn main() with panic { print(nest(1i64, 3)); }
// error: polymorphic recursion: `nest` uses itself with `T` = `Option<T>`
