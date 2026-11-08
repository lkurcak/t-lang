// What's in the `Some` a generic function gives is what's in one of the
// `Some`s it was given, not the first: here it can be the second's.
fn either<T: Copy>(a: Option<T>, b: Option<T>) -> Option<T> {
    match a {
        Some(_) => a,
        None => b,
    }
}

fn small(n: usize) -> Option<{ v: usize | v < 4 }> {
    if n < 4 { Some(n) } else { None }
}

fn at(a: [u8; 4], n: usize) -> u8 {
    match either(small(n), Some(9)) {
        Some(i) => a[i],
        None => 0,
    }
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
