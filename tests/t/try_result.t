// `?` gives what's in an `Ok`, or returns the `Err` from the function.
fn half(x: i64) -> Result<i64, bool> {
    if x % 2 == 0 { Ok(x / 2) } else { Err(x > 0) }
}

fn quarter(x: i64) -> Result<i64, bool> {
    let h = half(x)?;
    half(h)
}

/// A call made only for its error.
fn check(x: i64) -> Result<(), bool> {
    half(x)?;
    Ok(())
}

fn show(out: &mut std::io::Stdout, r: Result<i64, bool>) -> Result<(), std::io::Error> {
    match r {
        Ok(v) => out.line(&v),
        Err(b) => out.line(&b),
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    show(&mut out, quarter(8))?; // expect: 2
    show(&mut out, quarter(6))?; // expect: true
    show(&mut out, quarter(-3))?; // expect: false
    match check(4) {
        Ok(()) => out.line("ok")?, // expect: ok
        Err(_) => out.line("err")?,
    }
    Ok(())
}
