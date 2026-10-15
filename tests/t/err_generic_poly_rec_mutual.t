struct Wrap<T> { x: T }
fn ping<T>(x: T, n: int) -> int { if n == 0 { 0 } else { pong(Wrap { x }, n - 1) } }
fn pong<U>(y: U, n: int) -> int { ping(y, n) }
fn main() { print(ping(1, 3)); }
// error: polymorphic recursion: this use of `pong` with `U` = `Wrap<T>` leads back to `ping`
