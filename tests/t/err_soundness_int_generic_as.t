// Every `T` fits in... nothing in particular: a `u64` doesn't fit in an
// `i64`, nor an `i8` in a `u64`.
fn f<T: Int>(x: T) -> i64 with {} { x as i64 }

fn main() {}
// error: `as` may panic, but `f` doesn't say `with panic`
