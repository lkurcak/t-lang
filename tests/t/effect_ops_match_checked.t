// The operators are what `checked_*` and `panic("stop")` make of them.
fn add(a: i64, b: i64) -> i64 with panic {
    match checked_add(a, b) {
        Option::Some(c) => c,
        Option::None => panic("stop"),
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let big: i64 = 9223372036854775807;
    out.line(&(add(1, 2) == 1 + 2))?; // expect: true
    out.line(&checked_sub(-big - 1, 1).is_none())?; // expect: true
    out.line(&checked_mul(big, 2).is_none())?; // expect: true
    out.line(&checked_div(-big - 1, -1).is_none())?; // expect: true
    out.line(&checked_rem(7i64, 0).is_none())?; // expect: true
    out.line(&(checked_rem(-7i64, 2).unwrap() == -7 % 2))?; // expect: true
    out.line(&checked_neg(-big - 1).is_none())?; // expect: true
    Result::Ok(())
}
