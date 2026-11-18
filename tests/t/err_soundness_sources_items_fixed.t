// `X::Item = usize` lets `each` make items of its own: so `f` isn't
// walked knowing what `0..8` gives.
fn need8(_r: { r: usize | r < 8 }) {}

fn each<X: Iterator<Item = usize> + Discard, F: FnMut(X::Item) + Discard>(it: X, f: F) {
    let _ = it;
    let mut f = f;
    f(100);
}

fn main() {
    each(0..8, |x: usize| need8(x));
}
// error: can't prove `r < 8`
