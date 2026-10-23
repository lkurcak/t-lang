// `?` gives what's in an `Ok`, or returns the `Err` from the function.
fn half(x: i64) -> Result<i64, bool> {
    if x % 2 == 0 { Result::Ok(x / 2) } else { Result::Err(x > 0) }
}

fn quarter(x: i64) -> Result<i64, bool> {
    let h = half(x)?;
    half(h)
}

/// A call made only for its error.
fn check(x: i64) -> Result<(), bool> {
    half(x)?;
    Result::Ok(())
}

fn show(r: Result<i64, bool>) {
    match r {
        Result::Ok(v) => print(v),
        Result::Err(b) => print_bool(b),
    }
}

fn main() {
    show(quarter(8)); // expect: 2
    show(quarter(6)); // expect: true
    show(quarter(-3)); // expect: false
    match check(4) {
        Result::Ok(()) => print_str("ok"), // expect: ok
        Result::Err(_) => print_str("err"),
    }
}
