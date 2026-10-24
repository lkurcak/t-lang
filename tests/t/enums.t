// Enums: sum types. A value is exactly one variant, holding that variant's fields.

#[derive(Discard, Eq)]
enum Shape {
    Circle(i64),
    Rect(i64, i64),
    Empty,
}

fn area(s: Shape) -> i64 with panic {
    match s {
        Shape::Circle(r) => 3 * r * r,
        Shape::Rect(w, h) => w * h,
        Shape::Empty => 0,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&area(Shape::Circle(2)))?;     // expect: 12
    out.line(&area(Shape::Rect(3, 4)))?;    // expect: 12
    out.line(&area(Shape::Empty))?;         // expect: 0
    out.line(&(Shape::Rect(1, 2) == Shape::Rect(1, 2)))?;  // expect: true
    out.line(&(Shape::Rect(1, 2) == Shape::Circle(1)))     // expect: false
}
