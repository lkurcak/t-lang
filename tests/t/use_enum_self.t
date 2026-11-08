// `self` in a `use` group under an enum names the enum, as under a module
// it names the module: `use shapes::Shape::{self, Dot}`.
mod shapes {
    #[derive(Discard)]
    pub enum Shape { Dot, Line(u8) }
}

use shapes::Shape::{self, Dot, Line};

fn length(s: Shape) -> u8 {
    match s {
        Dot => 0,
        Line(n) => n,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let s: Shape = Line(3);
    out.line(&length(s))?;              // expect: 3
    out.line(&length(Dot))?;            // expect: 0
    out.line(&length(Shape::Line(5)))?; // expect: 5
    Ok(())
}
