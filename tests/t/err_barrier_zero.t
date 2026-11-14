// A barrier for no threads would let no one through, nor ever start a
// round: its count must be at least one.
use core::sync::Barrier;

fn main() {
    let _ = Barrier::new(0);
}
// error: can't prove `n >= 1` for argument `count` of `new`
