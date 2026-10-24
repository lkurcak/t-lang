// A branch that never ends tells what comes after that its condition
// didn't hold.
fn div(a: i64, d: { d: i64 | d > 0 }) -> i64 { a / d }

fn checked(a: i64, d: i64) -> i64 with panic {
    if d <= 0 {
        panic("stop");
    }
    div(a, d)
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&checked(9, 3))?; // expect: 3
    Result::Ok(())
}
