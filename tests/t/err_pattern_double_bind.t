#[derive(Discard)]
struct P { x: int, y: int }
fn main() {
    let P { x, y: x } = P { x: 1, y: 2 };
}
// error: `x` is bound more than once in the same pattern
