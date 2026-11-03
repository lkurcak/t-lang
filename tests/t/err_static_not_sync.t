// Every function can reach a static, so what's in it is shared: a cell,
// which isn't `Sync`, can't be in one.
use core::cell::Cell;

static COUNT: Cell<u32> = Cell::new(0);
// error: a static must be `Sync`, as every function can reach it, and `Cell<u32>` isn't

fn main() {}
