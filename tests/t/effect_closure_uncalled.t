// Making a closure doesn't run it: a closure that may panic can be made,
// and thrown away, where nothing may.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let z: i64 = 0;
    let c = || 1 / z;
    out.line(&1i64)?; // expect: 1
    Result::Ok(())
}
// warning: unused variable `c`
