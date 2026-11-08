// An invariant of a measure of a list the loop changes: each time round
// must make it hold again, from what the calls ensure.
use alloc::list::List;

fn fill(n: usize) -> List<u8> with alloc {
    let mut l: List<u8> = List::new();
    let mut i: usize = 0;
    while i < n invariant l.len() == i {
        l.push(0u8);
        l.push(0u8);
        i = i + 1;
    }
    l
}

fn main() {}
// error: can't prove `l.len() == i` for the loop's invariant, at the end of its body
