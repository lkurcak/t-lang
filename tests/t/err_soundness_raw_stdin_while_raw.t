// While the terminal is in raw mode, the input is read through the `Raw`,
// not the `Stdin` it holds.
use std::io::Error;

fn main(os: std::Os) -> Result<(), Error> with alloc {
    let mut input = os.stdin;
    let r = input.raw()?;
    let mut line = alloc::string::String::new();
    let _ = input.read_line(&mut line)?; // error: borrowed
    r.discard();
    line.discard();
    Ok(())
}
