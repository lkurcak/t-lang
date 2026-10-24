// `panic("stop")` aborts the program.
fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&1i64)?; // expect: 1
    panic("stop");
    out.line(&2i64)?;
    Result::Ok(())
}
// error: runtime error: panicked
// warning: unreachable code
