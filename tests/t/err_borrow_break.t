// A `break` leaves the loop's body, and `y` goes away with it, so the
// reference it gives the loop would dangle.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let r = loop {
        let y: i64 = 5;
        break &y;
    };
    out.line(&(*r))?;
    Result::Ok(())
}
// error: `y` doesn't live long enough: it goes out of scope here while still borrowed
