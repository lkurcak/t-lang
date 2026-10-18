#[derive(Discard)]
enum Shape { Circle(int) }
fn main() {
    let s = Shape::Square(1);
}
// error: `Shape` has no variant or method `Square`
