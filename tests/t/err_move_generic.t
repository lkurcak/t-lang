// A type parameter could be anything, so it's moved, not copied.
struct Pair<T> { a: T, b: T }

fn dup<T>(x: T) -> Pair<T> { Pair { a: x, b: x } }

fn main() {}
// error: use of moved value `x`
