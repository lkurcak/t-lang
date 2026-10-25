// A `&mut [T; N]` is invariant in `T`: an array of `&'static i64`s isn't
// one of shorter-lived references, which would let one be written where a
// long-lived one is read.
fn shorter<'a, 'b>(a: &'b mut [&'static i64; 1]) -> &'b mut [&'a i64; 1] { a }

fn main() {}
// error: lifetime may not live long enough
