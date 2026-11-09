// `push` changes the length, which the refinement of what `l` points to is
// about: proved again after the call, it doesn't hold.
use alloc::list::List;

fn grow(n: usize, l: &mut { l: List<u8> | l.len() == n }) with alloc {
    l.push(0);
}

fn main() with alloc {
    let mut l = List::filled(2, 0u8);
    grow(2, &mut l);
}
// error: can't prove `l.len() == n` for `*l` after the call to `push`
