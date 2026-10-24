// In a closure, `?` returns from the closure, not from the function it's
// in. Its result can be written or inferred.
fn half(x: i64) -> Result<i64, bool> {
    if x % 2 == 0 { Result::Ok(x / 2) } else { Result::Err(false) }
}

fn outer(x: i64) -> i64 with panic {
    let written = |y: i64| -> Result<i64, bool> { Result::Ok(half(y)? + 0) };
    let inferred = |y: i64| {
        let h = half(y)?;
        Result::Ok(h)
    };
    let a = match written(x) {
        Result::Ok(v) => v,
        Result::Err(_) => -1,
    };
    let b = match inferred(x) {
        Result::Ok(v) => v,
        Result::Err(_) => -1,
    };
    a * 10 + b
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&outer(4))?; // expect: 22
    out.line(&outer(3))?; // expect: -11
    Result::Ok(())
}
