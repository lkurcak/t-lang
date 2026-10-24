// Written out, polymorphic recursion is still polymorphic recursion.
#[derive(Discard)]
struct Wrap<T> { x: T }
fn grow<T>(n: i64) -> i64 with panic { if n == 0 { 0 } else { grow::<Wrap<T>>(n - 1) } }
fn main(os: std::Os) -> Result<(), std::io::Error> with panic { let mut out = os.stdout; out.line(&(grow::<i64>(2)))?;  Result::Ok(()) }
// error: polymorphic recursion: `grow` uses itself with `T` = `Wrap<T>`
