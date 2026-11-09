// A refined `let` can't be borrowed `&mut` other than straight for a
// call that can't keep the reference: through a variable, a write could
// break the refinement where nothing checks it.
use alloc::list::List;

fn main() with alloc {
    let mut l: { l: List<u8> | l.len() == 3 } = List::filled(3, 0);
    let r = &mut l;
    r.push(1);
    let _ = l[2];
}
// error: can't borrow `l` as `&mut`
