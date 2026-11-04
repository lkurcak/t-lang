// C calls it as it is: nothing could say what `T` is.
extern fn id<T>(x: T) -> T { x }
// error: an `extern fn` has no generic parameters

fn main() {}
