// A closure's parameter type is a projection an equality bound says:
// calling it with a `usize` is calling it with an `X::Item`. (Before 0.98.0
// the borrow checker didn't see that, and stopped.)
fn each<X: Iterator<Item = usize> + Discard, F: FnMut(X::Item) + Discard>(it: X, f: F) {
    let _ = it;
    let mut f = f;
    f(3);
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut seen = 0;
    each(0..8, |x: usize| seen = x);
    out.line(&seen) // expect: 3
}
