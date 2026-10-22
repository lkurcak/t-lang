// A list's values, as a slice: `len`, `get`, `first`, `last`, and taking it
// apart with `split_at`.
use alloc::list::List;
use core::slice::Split;

fn sum(s: &[int]) -> int with panic {
    let mut total = 0;
    let mut i = 0;
    while i < s.len() {
        total = total + *s.get(i).unwrap();
        i = i + 1;
    }
    total
}

fn main() with {alloc, panic} {
    let mut l: List<int> = List::new();
    for i in 1..5 {
        l.push(i * 10);
    }
    let s = l.as_slice();
    print(s.len());                  // expect: 4
    print_bool(s.is_empty());        // expect: false
    print(*s.first().unwrap());      // expect: 10
    print(*s.last().unwrap());       // expect: 40
    print_bool(s.get(4).is_none());  // expect: true
    print_bool(s.get(-1).is_none()); // expect: true
    print(sum(s));                   // expect: 100
    let Split { head, tail } = s.split_at(1).unwrap();
    print(head.len());                        // expect: 1
    print(sum(tail));                         // expect: 90
    print_bool(s.split_at(5).is_none());      // expect: true
    print(s.split_at(4).unwrap().tail.len()); // expect: 0
    print(sum(s));                            // expect: 100
    l.free();
}
