// A `main` with no `with` is a program that can't panic or allocate.
fn max(a: i64, b: i64) -> i64 { if a < b { b } else { a } }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&max(1, 2))?; // expect: 2
    let o = checked_add(9223372036854775807i64, 1);
    out.line(&o.is_none())?; // expect: true
    out.line(&checked_div(1i64, 0).is_none())?; // expect: true
    out.line(&checked_neg(5i64).is_some())?; // expect: true
    // A negative literal is a constant: `-` of a literal can't overflow.
    out.line(&(-9223372036854775807i64))?; // expect: -9223372036854775807
    Result::Ok(())
}
