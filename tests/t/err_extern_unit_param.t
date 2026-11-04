// `()` is a result C has, `void`, but no parameter.
extern fn f(x: ()) {}
// error: an `extern fn` takes only integers and `bool`s, which are what C's are, not `()`

fn main() {}
