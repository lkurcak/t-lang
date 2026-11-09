// A type parameter never stands for a slice: `swap` exchanges two `T`s,
// and two slices of different lengths can't be.
use core::mem::swap;

fn main() {
    let a: &mut [i64] = core::slice::empty_mut();
    let b: &mut [i64] = core::slice::empty_mut();
    swap(a, b);
}
// error: a type parameter that isn't `unsized` can't stand for the slice `[i64]`, which is only ever behind a reference
