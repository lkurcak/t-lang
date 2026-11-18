// `swap` may write what's behind either argument behind the other: the
// instantiation is both places' refinements, and `b`'s needs more. What it
// writes behind `b` is `a`'s value (docs/84-counting.md), which is that.
use core::mem::swap;

fn main() {
    let mut a: { v: u32 | v > 0 } = 1;
    let mut b: { v: u32 | v > 1 } = 2;
    swap(&mut a, &mut b);
    let _ = a;
    let _ = b;
}
// error: can't prove `v > 1` of what `swap` may write behind argument `b`
