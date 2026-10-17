struct P {}
fn main() {
    let p = P {};
    p.nope();
}
// error: `P` has no method `nope`
