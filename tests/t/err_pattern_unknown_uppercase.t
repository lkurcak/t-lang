// In a pattern a name that starts with an uppercase letter is a path, never
// a new variable (step 45): one that names nothing is an error, not a
// variable that matches everything, as `Empty` would be in Rust here.
#[derive(Discard)]
enum Shape {
    Empty,
    Dot(i64),
}

fn area(s: Shape) -> i64 {
    match s {
        Empty => 0,
// error: `Empty` isn't a variant in scope
        Shape::Dot(_) => 1,
    }
}

fn main() {
    let _ = area(Shape::Empty);
}
