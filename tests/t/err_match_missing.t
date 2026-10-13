enum Shape { Circle(int), Rect(int, int), Empty }
fn area(s: Shape) -> int {
    match s {
        Shape::Circle(r) => r,
        Shape::Empty => 0,
    }
}
fn main() {}
// error: non-exhaustive `match`: `Shape::Rect(_, _)` not covered
