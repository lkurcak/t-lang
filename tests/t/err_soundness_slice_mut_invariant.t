// A `&mut [T]` is invariant in `T`: a slice of `&'static int`s isn't one
// of shorter-lived references, which would let one be written where a
// long-lived one is read.
fn shorter<'a, 'b>(s: &'b mut [&'static int]) -> &'b mut [&'a int] { s }

fn main() {}
// error: lifetime may not live long enough
