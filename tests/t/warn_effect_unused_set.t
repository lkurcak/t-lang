// Each effect that's said but can't happen is a warning of its own.
fn one(n: i64) -> i64 with {alloc, panic} { n + 1 }

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    out.line(&one(1))?; // expect: 2
    Result::Ok(())
}
// warning: `one` says `with alloc`, but it can't allocate
