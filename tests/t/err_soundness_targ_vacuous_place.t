// A place that's `None` in one branch and `Some` of a list of numbers
// below 3 in the other: after, it's no list of numbers below 2.
use alloc::list::List;

fn below(n: usize) -> List<{ v: usize | v < n }> with alloc {
    let mut l: List<{ v: usize | v < n }> = List::new();
    if n > 0 {
        l.push(n - 1);
    }
    l
}

fn kept(b: bool) -> Option<List<{ v: usize | v < 2 }>> with alloc {
    let mut found = None;
    if b {
        found = Some(below(3));
    }
    found
}

fn main() with alloc {
    let _ = kept(true);
}
// error: can't prove `v < 2` for the result of `kept`
