// A box is `Discard` only if what it holds is.

use alloc::boxed::Box;

struct Fd { n: i64 }

#[derive(Discard)]
struct Holder { fd: Box<Fd> }

fn main() {}
// error: `Holder` can't be `Discard`: its field `fd` holds a `Box<Fd>`, which isn't
