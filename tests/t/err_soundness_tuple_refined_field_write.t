// A refined variable's refinement is an invariant: writing one field
// must keep it.
fn main() {
    let mut p: { p: (u8, u8) | p.0 < p.1 } = (1, 2);
    p.0 = 5;
    let _ = p;
}
// error: can't prove `p.0 < p.1`
