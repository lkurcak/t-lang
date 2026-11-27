// `Shape::Line(..)` in a measure says which variant a value is, and
// nothing of what's in it: a `Box3` doesn't have two corners.
#[derive(Copy, Discard)]
enum Shape { Line(u8, u8), Box3(u8, u8, u8) }

impl Shape {
    #[measure]
    fn corners(&self) -> u8 {
        match self {
            Shape::Line(..) => 2,
            Shape::Box3(..) => 8,
        }
    }
}

fn line() -> { s: Shape | s.corners() == 2 } { Shape::Box3(1, 2, 3) }

fn main() {
    let _ = line();
    let _ = Shape::Line(1, 2);
}
// error: can't prove `s.corners() == 2` for the result of `line`
