// `Result` comes from `core`, through the prelude.

fn div(a: i64, b: i64) -> Result<i64, bool> with panic {
    if b == 0 { Result::Err(false) } else { Result::Ok(a / b) }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let ok = div(6, 3);
    let err = div(1, 0);
    out.line(&ok.is_ok())?;   // expect: true
    out.line(&err.is_err())?; // expect: true
    out.line(&ok.unwrap_or(0))?;   // expect: 2
    out.line(&err.unwrap_or(-1))?; // expect: -1
    out.line(&ok.map(|x| x + 1).unwrap_or(0))?; // expect: 3
    out.line(&err.map_err(|e| !e).is_err())?; // expect: true
    out.line(&ok.and_then(|x| div(x, 2)).unwrap_or(0))?; // expect: 1
    out.line(&ok.and_then(|x| div(x, 0)).unwrap_or(-1))?; // expect: -1
    out.line(&err.or_else(|_e| div(9, 3)).unwrap_or(0))?; // expect: 3
    out.line(&ok.or_else(|_e| div(9, 3)).unwrap_or(0))?;  // expect: 2
    out.line(&err.or_else(|e| div(1, if e { 1 } else { 0 })).is_err())?; // expect: true
    out.line(&err.unwrap_or_else(|e| if e { 1 } else { 0 }))?; // expect: 0
    out.line(&ok.unwrap_or_else(|_e| 0))?; // expect: 2
    out.line(&ok.ok().unwrap_or(0))?;  // expect: 2
    out.line(&ok.err().is_none())?; // expect: true
    match err.as_ref() {
        Result::Ok(n) => out.line(&(*n))?,
        Result::Err(e) => out.line(&(*e))?, // expect: false
    }
    Result::Ok(())
}
