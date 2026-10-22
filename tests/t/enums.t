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

fn main() -> Shape with panic {
    print(area(Shape::Circle(2)));      // expect: 12
    print(area(Shape::Rect(3, 4)));     // expect: 12
    print(area(Shape::Empty));          // expect: 0
    print_bool(Shape::Rect(1, 2) == Shape::Rect(1, 2));  // expect: true
    print_bool(Shape::Rect(1, 2) == Shape::Circle(1));   // expect: false
    Shape::Rect(5, 6)                   // expect: Shape::Rect(5, 6)
}
