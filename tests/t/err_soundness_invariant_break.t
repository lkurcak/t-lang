// A `loop` left by a `break` where its invariant doesn't hold: what comes
// after the loop is how things were at that `break`, not the invariant
// (lemma I2 of docs/54-loop-invariants.md).

fn f(c: bool) -> u8 with {} {
    let a: [u8; 11] = [0; 11];
    let mut i: usize = 0;
    loop invariant i <= 10 {
        i = i + 20;
        if c { break; }
        i = 0;
    }
    a[i]
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
