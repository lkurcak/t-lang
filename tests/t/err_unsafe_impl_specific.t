// A promise is for the type, whatever its parameters are: one impl, with
// bounds, as a derive is.
use core::cell::Cell;

#[derive(Discard where T: Discard)]
struct L<T> { v: Cell<T> }

unsafe impl Sync for L<u32> {}
// error: `unsafe impl` must be for `L<T>`

fn main() {}
