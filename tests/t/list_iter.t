// `iter` borrows a list and gives a reference to each value, front to
// back. It's `Discard`, so `for` and the adapters take it.
use alloc::list::List;

fn sum(l: &List<int>) -> int with panic {
    let mut s = 0;
    for x in l.iter() {
        s = s + *x;
    }
    s
}

fn main() with {alloc, panic} {
    let mut l: List<int> = List::new();
    for i in 1..6 {
        l.push(i);
    }
    print(sum(&l));                                               // expect: 15
    print(l.iter().map(|x: &int| *x * 10).fold(0, |a, b| a + b)); // expect: 150
    print(l.iter().filter(|x: &&int| **x % 2 == 1).count());      // expect: 3
    let mut it = l.iter();
    it.next();
    print(*it.next().unwrap()); // expect: 2
    let e: List<int> = List::new();
    print_bool(e.iter().next().is_none()); // expect: true
    e.free();
    l.free();
}
