// What a function gives is one of what it was given, not the first of
// them: `keep` gives the default whatever the `Option` is.
fn keep<T: Copy>(o: Option<T>, d: T) -> T {
    let _ = o;
    d
}

fn small(n: usize) -> Option<{ v: usize | v < 4 }> {
    if n < 4 { Some(n) } else { None }
}

fn at(a: [u8; 4], n: usize) -> u8 { a[keep(small(n), 9)] }

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
