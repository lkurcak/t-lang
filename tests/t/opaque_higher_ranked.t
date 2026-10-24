// A bound with references in it is for every lifetime, so the closure
// returned for it is too.
fn same() -> impl Fn(&i64) -> &i64 + Discard { |x| x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let f = same();
    let a = 1;
    out.line(&(*f(&a)))?;                  // expect: 1
    {
        let b = 2;
        out.line(&(*f(&b)))?;              // expect: 2
    }
    Result::Ok(())
}
