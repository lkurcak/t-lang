#[derive(Discard)]
enum Shape { Rect(i64, i64) }
fn main() {
    let s = Shape::Rect(1);
}
// error: `Shape::Rect` has 2 fields but 1 was supplied
