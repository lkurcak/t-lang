fn main() -> Result<i64, std::io::Error> { Result::Ok(1) }
// error: `main` gives back `()`, or a `Result<(), E>` whose error is reported, not `Result<i64, Error>`
