// A slice to change: `get_mut`, `swap`, and two halves changed at once
// with `split_at_mut`, which share no values.
use alloc::list::List;
use core::slice::SplitMut;

fn bump(s: &mut [int]) with panic {
    let mut i = 0;
    while i < s.len() {
        match s.get_mut(i) {
            Option::Some(x) => *x = *x + 1,
            Option::None => {},
        }
        i = i + 1;
    }
}

fn main() with {alloc, panic} {
    let mut l: List<int> = List::new();
    for i in 0..4 {
        l.push(i);
    }
    let s = l.as_mut_slice();
    bump(s);
    bump(s);
    *s.get_mut(0).unwrap() = 100;
    print_bool(s.swap(0, 3)); // expect: true
    print_bool(s.swap(1, 4)); // expect: false
    let SplitMut { head, tail } = s.split_at_mut(2).unwrap();
    *head.get_mut(1).unwrap() = *tail.get_mut(0).unwrap();
    *tail.get_mut(1).unwrap() = 7;
    for x in l.iter() {
        print(*x);
    }
    // expect: 5
    // expect: 4
    // expect: 4
    // expect: 7
    l.free();
}
