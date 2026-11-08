// A struct's fields aren't ways to its type argument's values (a
// library's private fields would decide what its callers prove), so a
// `T` in one makes the type parameter not free.
#[derive(Copy, Discard)]
struct Pair<T> { a: T, b: T }

fn first<T: Copy>(p: Pair<T>, v: T) -> T {
    let _ = v;
    p.a
}

fn at(a: [u8; 4], i: { v: usize | v < 4 }) -> u8 { a[first(Pair { a: 9usize, b: 9 }, i)] }

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
