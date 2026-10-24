// Paths, `pub`, `use` with `as` and groups, `crate::`, `super::` and
// `self::`, and a module's private items seen from inside it.
mod geometry {
    #[derive(Copy, Discard)]
    pub struct Point { pub x: i64, pub y: i64 }

    pub enum Shape { Dot(Point), Line(Point, Point) }

    pub fn origin() -> Point { Point { x: 0, y: 0 } }

    pub fn len(s: Shape) -> i64 with panic {
        match s {
            Shape::Dot(_) => 0,
            Shape::Line(a, b) => self::dist(a.x, b.x) + dist(a.y, b.y),
        }
    }

    // Private: only `geometry` and the modules in it can call it.
    fn dist(a: i64, b: i64) -> i64 with panic { if a < b { b - a } else { a - b } }

    pub mod scale {
        use super::Point;

        pub fn twice(p: Point) -> Point with panic { Point { x: super::dist(0, p.x) * 2, y: p.y * 2 } }

        pub fn back() -> i64 { crate::helper() }
    }
}

use geometry::{Point, Shape, scale::twice as double};

fn helper() -> i64 { 7 }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let p = Point { x: 3, y: 4 };
    out.line(&geometry::len(Shape::Line(geometry::origin(), p)))?; // expect: 7
    out.line(&double(p).y)?;                                       // expect: 8
    out.line(&geometry::scale::back())?;                           // expect: 7
    let d = geometry::Shape::Dot(p);
    out.line(&geometry::len(d))?;                                  // expect: 0
    Result::Ok(())
}
