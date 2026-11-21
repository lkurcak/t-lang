// Of what's behind a `&mut`, a condition is of what it was at the call:
// emptied first, the list's `pop` gives `None` whatever it had.
use alloc::list::List;

fn last(l: &mut List<i64>) -> i64 with panic if l.len() == 0 {
    l.clear();
    l.pop().unwrap()
}

fn main() with alloc {
    let mut l = List::new();
    l.push(1);
    let _v = last(&mut l);
    l.discard();
}
// error: `unwrap` may panic when `l.len() == 0` doesn't hold, but `last` says `with panic if l.len() == 0`
