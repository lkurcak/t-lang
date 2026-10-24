// Every equality bound must hold, the ones that decide a parameter too:
// `U` is `i64`, from what `f` takes, and `f` gives a `bool`.
trait Go {
    fn go(&self) -> i64;
}

#[derive(Discard where F: Discard)]
struct W<F> { f: F }

impl<U, F: Fn(U) -> U> Go for W<F> {
    fn go(&self) -> i64 { 1 }
}

fn not_zero(x: i64) -> bool { x != 0 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let w = W { f: not_zero };
    out.line(&w.go())?;
    Result::Ok(())
}
// error: `W<fn(i64) -> bool>` doesn't implement `Go`
