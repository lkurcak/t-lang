// A `pub` function is only its package's, which is checked with it, so an
// effect it says but doesn't have is a warning, as for a private one.
mod m {
    pub fn one() -> i64 with panic { 1 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&m::one())?; // expect: 1
    Result::Ok(())
}
// warning: `one` says `with panic`, but it can't panic
