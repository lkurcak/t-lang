// The least `i64` modulo -1 would be 0, but a machine divides to find it,
// which overflows: an overflow, as in Rust.
fn rem(a: i64, b: i64) -> i64 with panic { a % b }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&rem(-9223372036854775807, -1))?; // expect: 0
    out.line(&rem(core::num::least(), -1))?;
    Result::Ok(())
}
// error: runtime error: integer overflow
