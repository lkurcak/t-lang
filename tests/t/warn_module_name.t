mod Geometry {
    pub fn one() -> i64 { 1 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&Geometry::one())?; // expect: 1
    Result::Ok(())
}
// warning: module `Geometry` should have a snake case name: `geometry`
