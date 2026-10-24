// A runtime error: the type system doesn't (yet!) know about zero.
fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&1i64)?; // expect: 1
    out.line(&(10i64 / (5 - 5)))
}
// error: runtime error: division by zero
