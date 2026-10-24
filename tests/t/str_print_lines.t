// `print_str` prints one line per call; a string with a newline in it
// prints two.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&"one\ntwo")?; // expect: one
    // expect: two
    out.line(&"")?; // expect: 
    Result::Ok(())
}
