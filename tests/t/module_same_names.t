// Two modules can each have a `Point` and a `make`; each path means its
// own. A module named `main` doesn't clash with `fn main`, nor with the
// functions nested in it.
mod a {
    #[derive(Discard)]
    pub struct Point { pub v: i64 }

    pub fn make() -> Point { Point { v: 1 } }
}

mod b {
    #[derive(Discard)]
    pub struct Point { pub v: bool }

    pub fn make() -> Point { Point { v: true } }
}

mod main {
    pub fn helper() -> i64 { 10 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let p: a::Point = a::make();
    let q: b::Point = b::make();
    out.line(&p.v)?;              // expect: 1
    out.line(&q.v)?;         // expect: true
    out.line(&helper())?;         // expect: 20
    out.line(&main::helper())?;   // expect: 10

    fn helper() -> i64 { 20 }
    Result::Ok(())
}
