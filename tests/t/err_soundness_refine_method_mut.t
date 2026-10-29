// A method taking `&mut self` borrows `d` mutably, without a `&mut` written.
trait Clear {
    fn clear(&mut self);
}

impl Clear for i64 {
    fn clear(&mut self) { *self = 0; }
}

fn f() -> i64 with {} {
    let mut d: i64 = 1;
    d.clear();
    10 / d
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&f())?;
    Result::Ok(())
}
// error: `/` may panic, but `f` doesn't say `with panic`
