// A bound saying another type's associated type is the iterator's type
// can make one: here a fresh range, which the function gives back instead
// of the one it's given.
use core::ops::Range;

trait Make {
    type Out;

    fn make() -> Self::Out;
}

#[derive(Discard)]
struct Fresh {}

impl Make for Fresh {
    type Out = Range<usize>;

    fn make() -> Range<usize> { 0..100 }
}

fn swap_in<I: Iterator + Discard, M: Make<Out = I>>(it: I) -> I {
    let _ = it;
    M::make()
}

fn main() {
    let a: [u8; 8] = [0; 8];
    for x in swap_in::<Range<usize>, Fresh>(0..8) {
        let _ = a[x];
    }
}
// error: can't prove
