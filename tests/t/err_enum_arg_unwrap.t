// A refinement doesn't go through a generic function: `unwrap` gives a
// `T`, of which nothing is known.
fn small(n: usize) -> Option<{ v: usize | v < 4 }> {
    if n < 4 { Some(n) } else { None }
}

fn f(n: usize) -> u8 with panic {
    let a: [u8; 4] = [1, 2, 3, 4];
    a[small(n).unwrap()]
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
