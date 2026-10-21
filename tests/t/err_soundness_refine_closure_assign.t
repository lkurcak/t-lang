// An assignment inside a closure must keep the refinement too.
fn main() {
    let mut i: { v: int | v > 0 } = 1;
    let mut c = || i = 0;
    c();
    print(i);
}
// error: can't prove `v > 0` for `i`
