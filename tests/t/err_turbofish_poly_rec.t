// Written out, polymorphic recursion is still polymorphic recursion.
#[derive(Discard)]
struct Wrap<T> { x: T }
fn grow<T>(n: int) -> int with panic { if n == 0 { 0 } else { grow::<Wrap<T>>(n - 1) } }
fn main() with panic { print(grow::<int>(2)); }
// error: polymorphic recursion: `grow` uses itself with `T` = `Wrap<T>`
