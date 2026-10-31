// A result's refinement names what's behind a `&mut` at return now: `r`
// is how long the list was, not how long it is after `pop`.
use alloc::list::List;

fn shrink(l: &mut List<i64>) -> { r: usize | r == l.len() } {
    let n = l.len();
    l.pop();
    n
}

fn main() {}
// error: can't prove `r == l.len()` for the result of `shrink`
