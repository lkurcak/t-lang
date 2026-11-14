// A slice can't be copied into itself, nor a list's values into the list
// through another way to them: what's changed and what's read are never
// the same values, so the copy can be one of the machine's, all at once.
use alloc::list::List;

fn main() with alloc {
    let mut l = List::filled(3, 1u8);
    l.as_mut_slice().copy_from_slice(l.as_slice());
}
// error: cannot borrow `l` while `l` is mutably borrowed
