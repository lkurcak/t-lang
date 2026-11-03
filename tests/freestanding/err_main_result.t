// What `main` gives back is reported by std's runtime, which a
// freestanding program doesn't have.
fn main() -> Result<(), u8> { Result::Ok(()) } // error: on the freestanding target, `main` gives back `()`
