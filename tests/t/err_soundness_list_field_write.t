// Nor can it change `len`, and with it which slots are said to hold values.
use alloc::list::List;

fn main() with {alloc, panic} {
    let mut l: List<int> = List::new();
    l.push(1);
    l.len = 5;
    print(*l.get(4).unwrap());
    l.free();
}
// error: field `len` of `List` is private to `alloc::list`
