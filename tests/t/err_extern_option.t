// Nor an enum: how it's laid out is the compiler's business.
extern fn next() -> Option<u8>;
// error: an `extern fn` gives back an integer, a `bool`, `()` or `!`, which are what C's can, not `Option<u8>`

fn main() {}
