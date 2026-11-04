// What `unsafe impl` promises is about a type's fields, which only its own
// package can see, and keep the promise of.
use core::cell::Cell;

unsafe impl<T: Send> Sync for Cell<T> {}
// error: `unsafe impl` of `Sync` must be in the package that declares `Cell`

fn main() {}
