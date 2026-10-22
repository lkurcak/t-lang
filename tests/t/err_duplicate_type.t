#[derive(Discard)]
struct P { x: i64 }
#[derive(Discard)]
enum P { A }
fn main() {}
// error: type `P` is already defined
