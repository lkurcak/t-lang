#[derive(Discard)]
struct Wrap<T> { x: T }
fn ping<T: Discard>(x: T, n: i64) -> i64 with panic { if n == 0 { 0 } else { pong(Wrap { x }, n - 1) } }
fn pong<U: Discard>(y: U, n: i64) -> i64 with panic { ping(y, n) }
fn main() with panic { print(ping(1i64, 3)); }
// error: polymorphic recursion: this use of `pong` with `U` = `Wrap<T>` leads back to `ping`
