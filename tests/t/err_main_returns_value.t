// What `main` gives back has nowhere to go but the exit status: `()`, or
// an error to report.
fn main() -> i64 { 1 }
// error: `main` gives back `()`, or a `Result<(), E>` whose error is reported, not `i64`
