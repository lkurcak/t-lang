// A list can't be extended by its own values: growing it would move them
// while they're being read.
use alloc::list::List;

fn main() with alloc {
    let mut l = List::filled(3, 1u8);
    l.extend_from_slice(l.as_slice());
}
// error: cannot borrow `l` as mutable while `l` is borrowed
