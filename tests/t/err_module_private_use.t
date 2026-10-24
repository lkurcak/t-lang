mod m {
    fn secret() -> i64 { 1 }
}

use m::secret;

fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&secret())?;  Result::Ok(()) }
// error: `secret` is private to `m`
