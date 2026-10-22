// A `List` is a dense, resizable array: `push` grows it, `pop` shrinks it,
// and `get` and `get_mut` see one value, or `None` past the end.
use alloc::list::List;

fn main() with {alloc, panic} {
    let mut l: List<int> = List::new();
    print_bool(l.is_empty()); // expect: true
    print(l.capacity());      // expect: 0
    let mut i = 0;
    while i < 10 {
        l.push(i * i);
        i += 1;
    }
    print(l.len());                  // expect: 10
    print_bool(l.capacity() >= 10);  // expect: true
    print(*l.get(3).unwrap());       // expect: 9
    print_bool(l.get(10).is_none()); // expect: true
    print_bool(l.get(-1).is_none()); // expect: true
    *l.get_mut(3).unwrap() += 100;
    print(*l.get(3).unwrap());           // expect: 109
    print_bool(l.get_mut(10).is_none()); // expect: true
    print(l.pop().unwrap());             // expect: 81
    print(l.pop().unwrap());             // expect: 64
    print(l.len());                      // expect: 8
    l.free();

    let mut m: List<Box<int>> = List::with_capacity(2);
    print(m.capacity()); // expect: 2
    m.push(Box::new(1));
    m.push(Box::new(2));
    m.push(Box::new(3));
    print(*m.pop().unwrap()); // expect: 3
    let mut e: List<int> = List::new();
    print_bool(e.pop().is_none()); // expect: true
    e.free();
    m.free();
}
