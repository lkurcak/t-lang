// A function generic in an iterator, given one by a closure too, can give
// back that one: what it gives is what either can give.
fn back<I: DoubleEndedIterator, G: FnOnce() -> I>(i: I, g: G) -> core::iter::Rev<I>
where
    I: Discard,
{
    let _ = i;
    g().rev()
}

fn main() {
    let a: [u8; 8] = [0; 8];
    for r in back(0..8, || 0..100) {
        let _ = a[r];
    }
}
// error: can't prove
