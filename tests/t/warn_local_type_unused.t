// A local type nothing reachable constructs is dead, like a top-level one.
// One in a function that's never used goes without saying.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    #[derive(Discard)]
    struct Unused { x: i64 }
    #[derive(Discard)]
    enum E { A, B }
    let _e = E::A;
    #[derive(Discard)]
    struct _OnPurpose {}
    out.line(&1i64)?; // expect: 1
    Result::Ok(())
}

fn dead() {
    #[derive(Discard)]
    struct InDead {}
}
// warning: struct `Unused` is never constructed
// warning: variant `E::B` is never constructed
// warning: function `dead` is never used
