// A call whose result keeps the borrow of `*l` (not call-scoped) can't be
// lent it: the refinement couldn't be proved again where the borrow ends.
use alloc::list::List;

fn keep<'a>(l: &'a mut List<u8>) -> &'a mut List<u8> { l }

fn shrink(n: usize, l: &mut { l: List<u8> | l.len() == n }) with alloc {
    let r = keep(l);
    let _ = r.pop();
}

fn main() with alloc {
    let mut l = List::filled(2, 0u8);
    shrink(2, &mut l);
}
// error: can't prove `l.len() == n` of what `keep` may write behind argument `l`
