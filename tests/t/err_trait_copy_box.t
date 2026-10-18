#[derive(Discard)]
struct B { b: Box<int> }
impl Copy for B {}
fn main() {}
// error: `B` can't be `Copy`: its field `b` holds a `Box<int>`, which isn't
