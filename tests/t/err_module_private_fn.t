mod m {
    pub fn open() -> i64 { secret() }

    fn secret() -> i64 { 1 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&m::secret())?;  Result::Ok(()) }
// error: `secret` is private to `m`
