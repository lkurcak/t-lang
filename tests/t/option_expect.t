// `expect` is `unwrap` with a reason.
fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let a: Option<i64> = Option::Some(3);
    out.line(&a.expect("a is some"))?; // expect: 3
    let r: Result<i64, bool> = Result::Ok(4);
    out.line(&r.expect("r is ok"))?; // expect: 4
    let b: Option<i64> = Option::None;
    b.expect("b is none");
    Result::Ok(())
}
// error: runtime error: panicked: b is none
