// A closure `keep` gives back can be called with anything later: it
// isn't walked knowing what the iterator gives.
fn need8(_r: { r: usize | r < 8 }) {}

fn keep<X: Iterator + Discard, F: FnMut(X::Item)>(it: X, f: F) -> F {
    let _ = it;
    f
}

fn main() {
    let mut f = keep(0..8, |x: usize| need8(x));
    f(100);
}
// error: can't prove `r < 8`
