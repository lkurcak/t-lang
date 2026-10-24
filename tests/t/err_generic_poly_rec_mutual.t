#[derive(Discard)]
struct Wrap<T> { x: T }
fn ping<T: Discard>(x: T, n: i64) -> i64 with panic { if n == 0 { 0 } else { pong(Wrap { x }, n - 1) } }
fn pong<U: Discard>(y: U, n: i64) -> i64 with panic { ping(y, n) }
fn main(os: std::Os) -> Result<(), std::io::Error> with panic { let mut out = os.stdout; out.line(&ping(1i64, 3))?;  Result::Ok(()) }
// error: polymorphic recursion: this use of `pong` with `U` = `Wrap<T>` leads back to `ping`
