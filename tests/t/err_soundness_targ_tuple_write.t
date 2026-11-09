// A refined tuple element, in a `let mut`: each write keeps it.
fn main() {
    let mut p: (u8, { v: u8 | v < 3 }) = (0, 1);
    p.1 = 5;
    let _ = p;
}
// error: can't prove `v < 3` for `p`
