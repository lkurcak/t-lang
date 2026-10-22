// A type parameter never stands for a slice: `swap` exchanges two `T`s,
// and two slices of different lengths can't be.
use core::mem::swap;

fn main() {
    let a: &mut [int] = core::slice::empty_mut();
    let b: &mut [int] = core::slice::empty_mut();
    swap(a, b);
}
// error: a type parameter can't stand for the slice `[int]`: it's only ever behind a reference
