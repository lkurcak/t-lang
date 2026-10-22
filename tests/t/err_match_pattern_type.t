#[derive(Discard)]
struct P { x: i64 }
#[derive(Discard)]
enum E { A }
fn main() {
    match (P { x: 1 }) { E::A => 1 };
}
// error: expected `P`, found `E`
