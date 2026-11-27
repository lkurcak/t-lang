// A variant measure's pattern may say `..` for a variant's fields: it's
// the `_`s it stands for (lemma P1 of docs/100-patterns.md), so it still
// only says which variant the value is.
#[derive(Copy, Discard)]
enum Shape { Dot, Line(u8, u8), Box3(u8, u8, u8) }

impl Shape {
    #[measure]
    fn corners(&self) -> u8 {
        match self {
            Shape::Dot => 0,
            Shape::Line(..) => 2,
            Shape::Box3(_, ..) => 8,
        }
    }
}

fn line() -> { s: Shape | s.corners() == 2 } { Shape::Line(1, 2) }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a: [u8; 3] = [4, 5, 6];
    out.line(&a[line().corners() as usize])?; // expect: 6
    let _ = Shape::Dot;
    let _ = Shape::Box3(1, 2, 3);
    Ok(())
}
