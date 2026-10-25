// A parameter's refinement is about the list when it's called: the list
// it points to can change after.
use alloc::list::List;

fn at(l: &mut List<i64>, i: { i: usize | i < l.len() }) -> i64 {
    l.pop();
    l[i]
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
