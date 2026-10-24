// A program of several files: `mod geo;` is `geo.t`, `mod area;` in it is
// `geo/area.t`, and `mod deep;` in the inline `mod inner { .. }` is
// `inner/deep.t`. They mean just what they would written inline.
mod geo;
pub mod shapes {
    //! Re-exports what's in a file.
    pub use crate::geo::area::square;
}
mod inner {
    pub mod deep;
}

use geo::Point;

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let p = geo::origin();
    let x = p.x;
    let q: Point = geo::at(3, 4);
    let area = geo::area::of(&q);
    let cq = geo::close(q);
    let cp = geo::close(p);
    out.line(&x)?;                      // expect: 0
    out.line(&area)?;                   // expect: 12
    out.line(&shapes::square(5))?;      // expect: 25
    out.line(&inner::deep::answer())?;  // expect: 42
    out.line(&cq)?;                     // expect: 7
    out.line(&cp)                       // expect: 0
}
