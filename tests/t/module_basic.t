// Paths, `pub`, `use` with `as` and groups, `crate::`, `super::` and
// `self::`, and a module's private items seen from inside it.
mod geometry {
    #[derive(Copy, Discard)]
    pub struct Point { pub x: int, pub y: int }

    pub enum Shape { Dot(Point), Line(Point, Point) }

    pub fn origin() -> Point { Point { x: 0, y: 0 } }

    pub fn len(s: Shape) -> int {
        match s {
            Shape::Dot(_) => 0,
            Shape::Line(a, b) => self::dist(a.x, b.x) + dist(a.y, b.y),
        }
    }

    // Private: only `geometry` and the modules in it can call it.
    fn dist(a: int, b: int) -> int { if a < b { b - a } else { a - b } }

    pub mod scale {
        use super::Point;

        pub fn twice(p: Point) -> Point { Point { x: super::dist(0, p.x) * 2, y: p.y * 2 } }

        pub fn back() -> int { crate::helper() }
    }
}

use geometry::{Point, Shape, scale::twice as double};

fn helper() -> int { 7 }

fn main() {
    let p = Point { x: 3, y: 4 };
    print(geometry::len(Shape::Line(geometry::origin(), p))); // expect: 7
    print(double(p).y);                                       // expect: 8
    print(geometry::scale::back());                           // expect: 7
    let d = geometry::Shape::Dot(p);
    print(geometry::len(d));                                  // expect: 0
}
