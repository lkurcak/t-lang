// Through a `&`, `x[i]` is `index`'s, and a method that takes `&mut self`
// can't be called on it.
use alloc::list::List;

fn main() with alloc {
    let mut l: List<List<u8>> = List::new();
    l.push(List::new());
    let shared = &l;
    if shared.len() == 1 {
        shared[0].push(1);
    }
}
// error: cannot borrow
