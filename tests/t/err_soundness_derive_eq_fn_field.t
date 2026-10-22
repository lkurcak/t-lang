// A derived `Eq` needs every field comparable, and a function never is,
// whatever the type parameters are.
#[derive(Eq, Discard)]
struct W<T> { f: fn(T) -> i64, n: i64 }

fn id(x: i64) -> i64 { x }

fn main() {
    print_bool(W { f: id, n: 1 } == W { f: id, n: 1 });
}
// error: `W` can't be `Eq`: its field `f` holds a `fn(T) -> i64`, which isn't (functions can't be compared)
