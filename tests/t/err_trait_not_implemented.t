trait Show { fn show(&self); }
#[derive(Discard)]
struct P {}
fn main() {
    let p = P {};
    p.show();
}
// error: `P` has no method `show`; trait `Show` has one, but `P` doesn't implement it
