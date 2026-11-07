// A loop that lends the list to a call ensuring its length stays and to
// one that grows it: the length isn't kept at the head.
use alloc::list::List;

fn f(l: &mut List<u8>) with alloc {
    let n = l.len();
    let mut i: usize = 0;
    while i < n {
        l.set(i, 1u8);
        l.push(0u8);
        l.pop();
        i = i + 1;
    }
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `set`
