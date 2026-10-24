// Called through the trait, as in `call`, nothing would check `by`.
trait Half {
    fn half(self, by: i64) -> i64;
}

impl Half for i64 {
    fn half(self, by: { b: i64 | b > 0 }) -> i64 { self / by }
}

fn call<T: Half>(x: T) -> i64 { x.half(0) }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&call(4))?;
    Result::Ok(())
}
// error: refinements can't be in a trait's methods yet
