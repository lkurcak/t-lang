// Saying a function may panic when it can't is a promise callers have to
// keep for nothing.
fn one() -> i64 with panic { 1 }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&one())?; // expect: 1
    Result::Ok(())
}
// warning: `one` says `with panic`, but it can't panic
