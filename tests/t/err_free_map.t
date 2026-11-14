// `map` takes a closure, which makes the value it gives: no free theorem
// (its result type is in the closure's bound). A closure written in the
// call says what it gives by its summary (docs/74), but a function given
// as a value isn't followed: what it gives could be anything.
fn small(n: usize) -> Option<{ v: usize | v < 4 }> {
    if n < 4 { Some(n) } else { None }
}

fn same(i: usize) -> usize { i }

fn at(a: [u8; 4], n: usize) -> u8 with panic { a[small(n).map(same).unwrap()] }

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
