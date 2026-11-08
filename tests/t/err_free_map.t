// `map` takes a closure, which makes the value it gives: no free theorem
// (its result type is in the closure's bound).
fn small(n: usize) -> Option<{ v: usize | v < 4 }> {
    if n < 4 { Some(n) } else { None }
}

fn at(a: [u8; 4], n: usize) -> u8 with panic { a[small(n).map(|i| i).unwrap()] }

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
