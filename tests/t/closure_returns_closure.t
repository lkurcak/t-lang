// A closure can make and return another: both borrow `b`, which outlives
// them.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let b: i64 = 2;
    let make = || || b;
    out.line(&make()())?;                // expect: 2
    Result::Ok(())
}
