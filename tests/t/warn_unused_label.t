// A label nothing names does nothing. The fix removes it.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut i: i64 = 0;
    'outer: while i < 3 {
        i += 1;
        if i == 2 { break; }
    }
    out.line(&i)?; // expect: 2
    Result::Ok(())
}
// warning: unused label `'outer`: no `break` or `continue` names it
