#[derive(Discard)]
struct P { x: i64, y: i64 }
fn main() {
    let p = P { x: 1 };
}
// error: missing field `y` in `P`
