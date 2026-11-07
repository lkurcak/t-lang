// All of the input at once, by `Stdin`'s own method, as a `ReadFile` has.
use alloc::string::String;
use std::io::Error;

fn main(os: std::Os) -> Result<(), Error> with alloc {
    let mut input = os.stdin;
    let mut out = os.stdout;
    let mut s = String::new();
    let n = input.read_to_string(&mut s)?;
    out.line(&n)?;         // expect: 8
    out.put(&s.as_str())?; // expect: one
    s.discard();           // expect: two
    Ok(())
}
// stdin: one
// stdin: two
