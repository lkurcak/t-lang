#[derive(Discard)]
enum W { Only(i64) }
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let W::Only(x) | W::Only(x) = W::Only(1);
    out.line(&x)?;
    Result::Ok(())
}
// warning: unreachable alternative
// expect: 1
