#[derive(Discard)]
struct P { x: i64, y: i64 }
fn main() {
    let P { x } = P { x: 1, y: 2 };
}
// error: pattern does not mention field `y`
