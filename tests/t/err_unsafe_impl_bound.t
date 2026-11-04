// What a promised `Sync` needs of its parameters can only be `Send` or
// `Sync`: proofs of them need nothing else.
use core::cell::Cell;

#[derive(Discard where T: Discard)]
struct L<T> { v: Cell<T> }

unsafe impl<T: Eq> Sync for L<T> {}
// error: `unsafe impl` of `Sync` can only need `Send` or `Sync`

fn main() {}
