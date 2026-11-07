// A loop lends a list's field to `set`, which keeps its length, and the
// whole struct to a function that may change it: the field's length isn't
// kept at the head.
use alloc::list::List;

struct S { l: List<u8> }

fn clear(s: &mut S) {
    s.l.clear();
}

fn f(s: &mut S) {
    let n = s.l.len();
    let mut i: usize = 0;
    while i < n {
        s.l.set(i, 1u8);
        clear(s);
        i = i + 1;
    }
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `set`
