// Without `unsafe`, `Sync` against a type's fields is refused: it's only
// derived, or promised.
use core::cell::Cell;

#[derive(Discard)]
struct L { v: Cell<u32> }

impl Sync for L {}
// error: `Sync` can only be derived, with `#[derive(Sync)]` on a struct or enum, so that whether a type is `Sync` follows from its fields alone, or promised with `unsafe impl`

fn main() {}
