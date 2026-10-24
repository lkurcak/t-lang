// A `move` closure owns `x`. A reference to it points into the closure, so
// it can't be returned from a call: the closure may be gone before it is,
// as here.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let x: i64 = 5;
    let r = {
        let c = move || &x;
        c()
    };
    out.line(&(*r))?;
    Result::Ok(())
}
// error: this closure returns a reference to what it captured, which only lasts as long as the call
