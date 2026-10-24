// A stream isn't `Copy`: there's one of each, so two parts of a program
// can't both own the output.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut a = os.stdout;
    let mut b = os.stdout;
    a.line(&1i64)?;
    b.line(&2i64)
}
// error: use of moved value `os.stdout`
