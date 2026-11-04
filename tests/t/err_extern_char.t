// A `char` is a `u32` that's a scalar value, which C doesn't promise.
extern fn f(c: char) {}
// error: an `extern fn` takes only integers and `bool`s, which are what C's are, not `char`

fn main() {}
