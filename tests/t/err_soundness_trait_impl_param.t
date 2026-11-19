// An impl's method can't ask more than its trait's: called through the
// trait, as in `call`, nothing would check `by` (docs/86-trait-contracts.md).
trait Half {
    fn half(self, by: i64) -> i64;
}

impl Half for i64 {
    fn half(self, by: { b: i64 | b > 0 }) -> i64 { self / by }
}

fn call<T: Half>(x: T) -> i64 { x.half(0) }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&call(4i64))?;
    Ok(())
}
// error: can't prove `b > 0` from what trait `Half` asks of `by`
