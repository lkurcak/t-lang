// A variable has one type. `id` is `fn(i64) -> i64` once it's called with an
// i64, so it can't take a bool too. Closures stay monomorphic even with
// generics: only a top-level `fn id<T>(x: T) -> T` can be used at both.
fn main() {
    let id = |x| x;
    print(id(1));
    print_bool(id(true));
}
// error: expected `{integer}`, found `bool`
