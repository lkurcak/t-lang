// A `//!` after something in its module documents nothing.
mod shapes {
    pub fn one() -> i64 { 1 }

    //! Too late.
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    //! Not a module.
    out.line(&shapes::one())?; // expect: 1
    Result::Ok(())
}
// warning: unused doc comment: a `//!` one documents the module it's at the start of, so it goes before anything else in the module
// warning: unused doc comment: a `//!` one documents the module it's at the start of, so it goes before anything else in the module
