#[derive(Discard)]
enum Shape { Circle(i64) }
fn main() {
    let s = Shape::Square(1);
}
// error: `Shape` has no variant or method `Square`
