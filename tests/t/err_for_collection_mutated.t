// Going through `&l` borrows `l` for the whole loop.
use alloc::list::List;

fn main() with alloc {
    let mut l: List<i64> = List::new();
    l.push(1);
    for x in &l {
        l.push(*x); // error: cannot borrow `l` as mutable while `l` is borrowed
    }
}
