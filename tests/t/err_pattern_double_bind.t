#[derive(Discard)]
struct P { x: i64, y: i64 }
fn main() {
    let P { x, y: x } = P { x: 1, y: 2 };
}
// error: `x` is bound more than once in the same pattern
