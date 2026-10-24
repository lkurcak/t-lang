// Slices compare value by value, when their values do.
use alloc::list::List;

fn main() with alloc {
    let mut a: List<i64> = List::new();
    a.push(1);
    a.push(2);
    let mut b: List<i64> = List::new();
    b.push(1);
    print_bool(a.as_slice() == b.as_slice()); // expect: false
    b.push(2);
    print_bool(a.as_slice() == b.as_slice()); // expect: true
    a.free();
    b.free();
}
