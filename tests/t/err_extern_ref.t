// T makes no promise of how a reference is laid out, nor what it points
// to: only C's scalars cross.
extern fn put(s: &str);
// error: an `extern fn` takes only integers and `bool`s, which are what C's are, not `&str`

fn main() {}
