// What a comparison is waits for its operands' type: here they turn out to
// be integers, so it's the machine's `<`, not a call of `Ord::lt`, as
// `t mono` shows.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let less = |a, b| a < b;
    out.line(&less(1i64, 2))?; // expect: true
    out.line(&less(2i64, 1))?; // expect: false
    Result::Ok(())
}
