// A list's `discard` doesn't read what its values borrow, but nothing says
// so, and it's checked as one that might: declare `x` first.
use alloc::list::List;

fn main() with alloc {
    let mut l = List::new();
    let x: i64 = 5;
    l.push(&x);
}
// error: `x` doesn't live long enough
