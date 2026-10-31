// `Signed` and `Unsigned` are sealed too, and what the checker gives them
// is all they have: an unsigned type is never `Signed`, so an impl for
// every `T: Signed` doesn't overlap one for `u8` (see
// docs/29-closed-and-sealed-traits.md).
trait Sign {
    fn sign(&self) -> i64;
}

impl<T: Signed> Sign for T {
    fn sign(&self) -> i64 {
        if *self < 0 { -1 } else { 1 }
    }
}

impl Sign for u8 {
    fn sign(&self) -> i64 { 0 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&(-5i32).sign())?; // expect: -1
    out.line(&5u8.sign())?;     // expect: 0
    Result::Ok(())
}
