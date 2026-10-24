// `main` says what the program may do, like any function.

use alloc::boxed::Box;

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let b = Box::new(1i64);
    out.line(&(*b))?;
    Result::Ok(())
}
// error: `Box::new` may allocate, but `main` doesn't say `with alloc`
