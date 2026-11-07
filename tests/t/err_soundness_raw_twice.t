// There's never more than one `Raw`: each holds the one `Stdin` by `&mut`
// (lemma T3 of docs/49-time-entropy-terminal.md).
use std::io::Error;

fn main(os: std::Os) -> Result<(), Error> {
    let mut input = os.stdin;
    let a = input.raw()?;
    let b = input.raw()?; // error: borrowed
    a.discard();
    b.discard();
    Ok(())
}
