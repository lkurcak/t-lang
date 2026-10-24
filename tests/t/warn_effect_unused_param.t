// An effect parameter said but never had: `f` is never called.
fn ignore<effect e, F: Fn() with e + Discard>(f: F) with e {}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    ignore(|| {});
    out.line(&1i64)?; // expect: 1
    Result::Ok(())
}
// warning: unused variable `f`
// warning: `ignore` says `with e`, but nothing in it has the effects `e`
