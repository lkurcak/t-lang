// `swap`'s indices must be proved in range.
use alloc::list::List;

fn main() with alloc {
    let mut l: List<u8> = List::filled(3, 0);
    l.swap(0, 3);
}
// error: can't prove `j < self.len()` for argument `j` of `swap`
