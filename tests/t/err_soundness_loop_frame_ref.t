// A loop that lends the list by a `&mut` kept in a local, not straight to a
// call: what's written through it isn't known, so the length isn't kept.
use alloc::list::List;

fn f(l: &mut List<u8>) {
    let n = l.len();
    let mut i: usize = 0;
    while i < n {
        l.set(i, 1u8);
        let r = &mut *l;
        r.clear();
        i = i + 1;
    }
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `set`
