// A `u8` is not `Signed`.
fn f<T: Signed>(x: T) -> T { x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(1u8))?;
    Result::Ok(())
}
// error: `f` needs its `T` to implement `Signed`, and `u8` doesn't
