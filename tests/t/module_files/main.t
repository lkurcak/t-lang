// A program of several files: `mod geo;` is `geo.t`, `mod area;` in it is
// `geo/area.t`, and `mod deep;` in the inline `mod inner { .. }` is
// `inner/deep.t`. They mean just what they would written inline.
mod geo;
/// Re-exports what's in a file.
pub mod shapes {
    pub use crate::geo::area::square;
}
mod inner {
    pub mod deep;
}

use geo::Point;

fn main() {
    let p = geo::origin();
    print(p.x);                    // expect: 0
    let q: Point = geo::at(3, 4);
    print(geo::area::of(&q));      // expect: 12
    print(shapes::square(5));      // expect: 25
    print(inner::deep::answer());  // expect: 42
    print(geo::close(q));          // expect: 7
    print(geo::close(p));          // expect: 0
}
