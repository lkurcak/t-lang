// A refinement can only name `int`s and `bool`s.
struct Tagged<T> { t: T, n: { v: int | v == t } }

fn main() {}
// error: a refinement can only name `int`s and `bool`s, and `t` is a `T`
