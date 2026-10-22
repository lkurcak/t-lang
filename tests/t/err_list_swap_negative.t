// An index of `swap` isn't negative: its type says so.
use alloc::list::List;

fn main() with {alloc, panic} {
    let mut l: List<int> = List::new();
    l.push(1);
    l.swap(-1, 0);
    l.free();
}
// error: can't prove `i >= 0` for argument `i` of `swap`
