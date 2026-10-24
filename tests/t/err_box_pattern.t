use alloc::boxed::Box;

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let b = Box::new(1);
    match b {
        Box::new(x) => out.line(&x)?,
    }
    Result::Ok(())
}
// error: `Box` isn't an enum
