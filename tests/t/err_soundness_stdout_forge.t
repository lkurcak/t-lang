// A stream can't be made: only `main` is given one, so a function that
// wasn't handed it can't write.
fn sneaky() -> Result<(), std::io::Error> {
    let mut out = std::io::Stdout {};
    out.line(&1i64)
}

fn main() {
    let _ = sneaky();
}
// error: `Stdout` is intrinsic: only its own functions make one or take one apart
