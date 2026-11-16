// What a range gives is kept only while it changes by its own methods: a
// function lent it can put another range there.
use core::iter::Rev;
use core::ops::Range;

fn widen(it: &mut Rev<Range<usize>>) { *it = (0..100).rev(); }

fn main() {
    let a: [u8; 8] = [0; 8];
    let mut it = (0..8).rev();
    while let Some(r) = it.next() {
        let _ = a[r];
        widen(&mut it);
    }
}
// error: can't prove
