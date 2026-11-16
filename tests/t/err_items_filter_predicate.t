// `filter` gives what its iterator gives: what its closure said of each is
// in the adapter's body, not its declaration, so it isn't known
// (docs/82-item-invariants.md).
fn need(_x: { x: usize | x > 2 }) {}

fn main() {
    for r in (0..8).filter(|x| *x > 2) {
        need(r);
    }
}
// error: can't prove `x > 2` for argument `_x` of `need`
