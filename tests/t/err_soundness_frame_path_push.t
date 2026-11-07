// A path call lending the list `&mut` straight keeps its length only if
// it ensures so: `push` doesn't.
use alloc::list::List;

fn f(l: &mut List<u8>) with alloc {
    let n = l.len();
    let mut i: usize = 0;
    while i < n {
        List::push(&mut *l, 0u8);
        List::set(&mut *l, n - 1, 1u8);
        i = i + 1;
    }
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `set`
