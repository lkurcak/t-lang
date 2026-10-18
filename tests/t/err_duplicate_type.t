#[derive(Discard)]
struct P { x: int }
#[derive(Discard)]
enum P { A }
fn main() {}
// error: type `P` is already defined
