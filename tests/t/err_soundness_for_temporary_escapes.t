// The head's temporaries live as long as the `for`'s statement, no longer:
// a reference into one can't be kept after it.
fn squares() -> [u32; 4] { [1, 4, 9, 16] }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let zero: u32 = 0;
    let mut last = &zero;
    for x in squares().iter() {
        last = x;
    }
    out.line(last)?;
    Ok(())
}
// error: a temporary value is dropped here while still borrowed
