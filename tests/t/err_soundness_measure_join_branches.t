// After an `if`, a place each branch left a different object at is a new
// one, whose measures are, on each path, that path's: here `l.len()` is 1
// or 0, so `l[0]` can't be proved.
use alloc::list::List;

fn first(l: &mut List<i64>, flag: bool) -> i64 with alloc {
    if flag {
        l.clear();
        l.push(1);
    } else {
        l.clear();
    }
    l[0]
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
