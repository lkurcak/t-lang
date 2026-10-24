// An abstract type from a file can't be forged outside it, any more than
// one from an inline module.
mod fd;
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let f = fd::Fd { raw: 3 }; // error: field `raw` of `Fd` is private to `fd`
    out.line(&fd::close(f))?;
    Result::Ok(())
}
