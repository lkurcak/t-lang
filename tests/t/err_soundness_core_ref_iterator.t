// `&mut I` is an iterator, borrowing `I`; it can't outlive it.
fn keep<'a>(it: &'a mut core::ops::Range<i64>) -> &'a mut core::ops::Range<i64> { it }

fn main() {
    let r;
    {
        let mut it = 0..3;
        r = keep(&mut it);
    }
    print(r.next().unwrap_or(0));
}
// error: `it` doesn't live long enough
