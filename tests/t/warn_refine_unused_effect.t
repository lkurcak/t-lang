// Proving the operators can't panic leaves nothing that may.
fn add_one(x: { v: i64 | v < 10 }) -> i64 with panic { x + 1 }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&add_one(1))?; // expect: 2
    Result::Ok(())
}
// warning: `add_one` says `with panic`, but it can't panic
