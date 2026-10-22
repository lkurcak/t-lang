#[derive(Discard)]
enum Shape { Circle(i64), Rect(i64, i64), Empty }
fn area(s: Shape) -> i64 {
    match s {
        Shape::Circle(r) => r,
        Shape::Empty => 0,
    }
}
fn main() {}
// error: non-exhaustive `match`: `Shape::Rect(_, _)` not covered
