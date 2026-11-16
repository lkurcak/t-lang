// A function lent an iterator, not one of its own methods, can leave
// another there: what it gave before says nothing of what it gives now.
use core::iter::Rev;
use core::ops::Range;

fn widen(it: &mut Rev<Range<usize>>) { *it = (0..100).rev(); }

fn main() {
    let a: [u8; 8] = [0; 8];
    let mut it = (0..8).rev();
    widen(&mut it);
    match it.next() {
        Some(x) => {
            let _ = a[x];
        },
        None => {},
    }
}
// error: can't prove
