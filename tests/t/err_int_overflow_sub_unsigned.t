fn sub(a: usize, b: usize) -> usize with panic { a - b }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&sub(1, 2))?;
    Result::Ok(())
}
// error: runtime error: integer overflow
