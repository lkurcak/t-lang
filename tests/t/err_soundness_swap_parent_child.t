// A list and its own tail, through the borrow the `match` holds.

use core::mem::swap;

#[derive(Discard)]
enum List { Cons(int, Box<List>), Nil }

fn main() {
    let mut l = List::Cons(1, Box::new(List::Cons(2, Box::new(List::Nil))));
    match &mut l {
        List::Cons(_, t) => swap(&mut l, &mut **t),
        List::Nil => {}
    }
}
// error: cannot borrow `l` as mutable more than once at a time
