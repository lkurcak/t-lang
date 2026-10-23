// `expect` is `unwrap` with a reason.
fn main() with panic {
    let a: Option<i64> = Option::Some(3);
    print(a.expect("a is some")); // expect: 3
    let r: Result<i64, bool> = Result::Ok(4);
    print(r.expect("r is ok")); // expect: 4
    let b: Option<i64> = Option::None;
    b.expect("b is none");
}
// error: runtime error: panicked: b is none
