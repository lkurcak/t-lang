// `swap` exchanges two values, and says if both indices were in range;
// `truncate` and `clear` throw values away from the end, and keep the
// buffer.

use alloc::boxed::Box;
use alloc::list::List;

fn show(l: &List<i64>) {
    for x in l.iter() {
        print(*x);
    }
}

fn main() with {alloc, panic} {
    let mut l: List<i64> = List::new();
    for i in 0..4 {
        l.push(i);
    }
    print_bool(l.swap(0, 3)); // expect: true
    print_bool(l.swap(1, 1)); // expect: true
    print_bool(l.swap(2, 4)); // expect: false
    show(&l);
    // expect: 3
    // expect: 1
    // expect: 2
    // expect: 0
    l.truncate(9);
    print(l.len()); // expect: 4
    l.truncate(2);
    show(&l);
    // expect: 3
    // expect: 1
    let cap = l.capacity();
    l.clear();
    print_bool(l.is_empty());        // expect: true
    print_bool(l.capacity() == cap); // expect: true
    l.push(7);
    show(&l); // expect: 7
    l.free();

    // `swap` needs no `Discard`.
    let mut b: List<Box<i64>> = List::new();
    b.push(Box::new(1));
    b.push(Box::new(2));
    b.swap(0, 1);
    print(**b.get(0).unwrap()); // expect: 2
    b.free();
}
