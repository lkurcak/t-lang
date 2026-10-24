#[derive(Discard)]
struct P { n: i64 }
impl P {
    fn take(self) -> i64 { self.n }
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let p = P { n: 1 };
    let r = &p;
    out.line(&r.take())?;
    Result::Ok(())
}
// error: cannot move out of `*r`, which is behind a reference
