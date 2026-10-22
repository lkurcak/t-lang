#[derive(Discard)]
struct B { b: Box<i64> }
impl Copy for B {}
fn main() {}
// error: `B` can't be `Copy`: its field `b` holds a `Box<i64>`, which isn't
