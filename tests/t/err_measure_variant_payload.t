// A measure can say which variant a value is, but not what's in it.
enum Shape {
    Dot,
    Line(u8),
}

impl Shape {
    #[measure]
    fn long(&self) -> bool {
        match self {
            Shape::Dot => false,
            Shape::Line(n) => *n > 3,
        }
    }
}

fn main() {}
// error: a measure's `match` can only say which variant
