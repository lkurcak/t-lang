// Nor may generic code count on a type being signed: `0 - 1` doesn't fit
// in a `u8`.
fn f<T: Int>() -> T with {} { 0 - 1 }

fn main() {}
// error: `-` may panic, but `f` doesn't say `with panic`
