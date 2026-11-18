// A closure given a `&mut X` can put another iterator there: so what
// `redo` gives back isn't only what it was given (lemma P1 of docs/83).
fn redo<X: Iterator, F: FnOnce(&mut X)>(x: X, f: F) -> X {
    let mut y = x;
    f(&mut y);
    y
}

fn main() {
    let a: [u8; 8] = [0; 8];
    for r in redo(0..8, |it: &mut core::ops::Range<usize>| *it = 100..101) {
        let _ = a[r];
    }
}
// error: can't prove `i < self.len()`
