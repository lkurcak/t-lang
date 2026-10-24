// A program's own types can be `Display`, built from what's in them with
// `?`, for any sink; and generic code can take any `Display`.
use alloc::string::String;
use core::fmt::{Display, Write};

#[derive(Discard)]
struct Point { x: i64, y: i64 }

impl Display for Point {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e {
        w.write_char('(')?;
        w.put(&self.x)?;
        w.write_str(", ")?;
        w.put(&self.y)?;
        w.write_char(')')
    }
}

#[derive(Discard)]
enum Shape { Dot(Point), Line(Point, Point) }

impl Display for Shape {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e {
        match self {
            Shape::Dot(p) => {
                w.write_str("dot at ")?;
                p.fmt(w)
            },
            Shape::Line(a, b) => {
                w.write_str("line from ")?;
                w.put(a)?;
                w.write_str(" to ")?;
                w.put(b)
            },
        }
    }
}

/// Each of `xs`, with `sep` between.
fn join<D: Display, W: Write>(xs: &[D], sep: &str, w: &mut W) -> Result<(), W::Error> with W::e {
    let mut first = true;
    for x in xs.iter() {
        if !first {
            w.write_str(sep)?;
        }
        first = false;
        w.put(x)?;
    }
    Result::Ok(())
}

fn main() with alloc {
    let s = String::of(&Shape::Line(Point { x: 1, y: -2 }, Point { x: 30, y: 4 }));
    print_str(s.as_str()); // expect: line from (1, -2) to (30, 4)
    s.free();
    let d = String::of(&Shape::Dot(Point { x: 0, y: 0 }));
    print_str(d.as_str()); // expect: dot at (0, 0)
    d.free();

    let mut l = alloc::list::List::new();
    l.push(3u16);
    l.push(1);
    l.push(4);
    let mut out = String::new();
    let Result::Ok(()) = join(l.as_slice(), ", ", &mut out);
    print_str(out.as_str()); // expect: 3, 1, 4
    out.free();
    l.free();
}
