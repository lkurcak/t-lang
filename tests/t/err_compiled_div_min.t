// The least `i64` divided by -1 is one past the greatest: an overflow,
// which C would make undefined behaviour, and the backend checks for.
fn div(a: i64, b: i64) -> i64 with panic { a / b }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&div(-9223372036854775807, -1))?; // expect: 9223372036854775807
    out.line(&div(core::num::least(), -1))?;
    Result::Ok(())
}
// error: runtime error: integer overflow
