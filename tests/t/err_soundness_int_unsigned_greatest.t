// Of a `T: Unsigned` the greatest may be 255.
fn f<T: Unsigned>() -> T { 255 + 1 }

fn main() {}
// error: `+` may panic, but `f` doesn't say `with panic`
