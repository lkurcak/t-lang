// A refined `let` lent to a call that ensures nothing of what it leaves:
// proved again after the call, the refinement doesn't follow.
use alloc::list::List;

fn clear(l: &mut List<u8>) {
    while l.len() > 0 invariant true {
        let _ = l.pop();
    }
}

fn main() with alloc {
    let mut l: { l: List<u8> | l.len() == 3 } = List::filled(3, 0);
    clear(&mut l);
    let _ = l[2];
}
// error: can't prove `l.len() == 3` for `l` after the call to `clear`
