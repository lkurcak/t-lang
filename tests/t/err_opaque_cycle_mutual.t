// `f` returns what holds what `g` returns, which holds what `f` returns.
fn f(n: i64) -> impl Fn() -> i64 + Discard {
    let h = g(n);
    move || h()
}

fn g(n: i64) -> impl Fn() -> i64 + Discard {
    let h = f(n);
    move || h()
}

fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&f(1)())?;  Result::Ok(()) }
// error: would contain itself
