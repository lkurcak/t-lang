#[derive(Discard)]
struct P { x: int, y: int }
fn main() {
    let P { x } = P { x: 1, y: 2 };
}
// error: pattern does not mention field `y`
