// `if` is an expression; `else if` chains.

fn sign(n: i64) -> i64 {
    if n < 0 { -1 } else if n == 0 { 0 } else { 1 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&sign(-7))?;    // expect: -1
    out.line(&sign(0))?;     // expect: 0
    out.line(&sign(42))?;    // expect: 1
    if sign(5) == 1 {
        out.line(&100i64)?;     // expect: 100
    }
    if false { out.line(&999i64)? }
    Result::Ok(())
}
