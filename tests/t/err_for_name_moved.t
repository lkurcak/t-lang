// The body can move the iterator out, but then the loop has none to go on
// with: unless it leaves, as `return` or `break` do.
use core::slice::Iter;

fn keep<'a>(_it: Iter<'a, u8>) {}

fn main() {
    let a = [1u8, 2];
    for _x in it: a.iter() {
        keep(it);
    }
}
// error: use of moved value `it`
