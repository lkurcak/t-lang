// `r.ok()` is a `Some` only if `r` was an `Ok`, but a `None` says nothing
// of `r`: what a free theorem says of the arguments holds only where the
// result's part it's about is there.
fn sorted(n: usize) -> Result<{ v: usize | v < 4 }, { e: usize | e >= 4 }> {
    if n < 4 { Ok(n) } else { Err(n) }
}

fn at(a: [u8; 4], n: usize) -> u8 {
    let r = sorted(n);
    match r.ok() {
        Some(_) => 0,
        None => match r {
            Ok(i) => a[i],
            Err(e) => a[e],
        },
    }
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
