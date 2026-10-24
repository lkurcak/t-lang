// `break 'outer` leaves both loops, and `y`, in the outer one's body, goes
// away with them.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let r;
    'outer: loop {
        let y: i64 = 5;
        loop {
            r = &y;
            break 'outer;
        }
    }
    out.line(&(*r))?;
    Result::Ok(())
}
// error: `y` doesn't live long enough
