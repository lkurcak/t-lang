// Lengths and indices are `usize`s. How wide a `usize` is depends on the
// machine, 32 or 64 bits, and what the checker proves holds on both: a
// `u32` always fits in one, and a `usize` in a `u64`.
use alloc::list::List;

fn last(s: &[u8]) -> Option<&u8> {
    let n = s.len();
    if n > 0 { s.get(n - 1) } else { Option::None }
}

fn index(i: u32) -> usize { i as usize }

fn main() with {alloc, panic} {
    let mut l: List<u8> = List::new();
    l.push(1);
    l.push(2);
    l.push(3);
    print(l.len());                         // expect: 3
    print(*last(l.as_slice()).unwrap());    // expect: 3
    print(*l.get(index(1)).unwrap());       // expect: 2
    l.free();
}
