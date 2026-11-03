// `Sync` is only derived, so that whether a type is follows from its fields.
use core::cell::Cell;

#[derive(Discard)]
struct Counter { n: Cell<u32> }

impl Sync for Counter {}
// error: `Sync` can only be derived

fn main() {}
