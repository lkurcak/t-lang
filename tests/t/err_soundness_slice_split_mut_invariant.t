// A tuple holding a `&mut [T]` is invariant in `T`, as the slice is:
// `split_at_mut`'s halves could be written through.
fn shorter<'a, 'b>(
    s: (&'b mut [&'static i64], &'b mut [&'static i64]),
) -> (&'b mut [&'a i64], &'b mut [&'a i64]) {
    s
}

fn main() {}
// error: lifetime may not live long enough
