// A loop that lends the list to a call ensuring its length stays, and
// assigns it too: the length isn't kept at the head.
use alloc::list::List;

fn f(l: &mut List<u8>) {
    let n = l.len();
    let mut i: usize = 0;
    while i < n {
        l.set(i, 1u8);
        *l = List::new();
        i = i + 1;
    }
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `set`
