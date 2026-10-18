#[derive(Discard)]
struct P {}
impl P { fn new() -> P { P {} } }
fn main() {
    let p = P {};
    p.new();
}
// error: `new` doesn't take `self`, so it can't be called with `.`; call it as `P::new(..)`
