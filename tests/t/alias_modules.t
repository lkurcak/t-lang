// An alias means what it stands for where it's declared: `m::P` is `m`'s
// `Point`, wherever it's used. Its uses go through `use` and paths like
// any type's.
mod m {
    pub struct Point { pub x: i64 }
    pub type P = Point;
    pub fn origin() -> P { Point { x: 0 } }
}

struct Point { x: bool }

use m::P;

fn shift(p: P) -> m::Point { m::Point { x: p.x + 1 } }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let p: P = shift(m::origin());
    out.line(&p.x)?; // expect: 1
    let q = Point { x: true };
    out.line(&q.x)?; // expect: true
    Result::Ok(())
}
