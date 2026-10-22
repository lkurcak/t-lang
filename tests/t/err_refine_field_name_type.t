// A refinement can only name `i64`s and `bool`s.
struct Tagged<T> { t: T, n: { v: i64 | v == t } }

fn main() {}
// error: a refinement can only name integers and `bool`s, and `t` is a `T`
