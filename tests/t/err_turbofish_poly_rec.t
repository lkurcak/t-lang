// Written out, polymorphic recursion is still polymorphic recursion.
struct Box<T> { x: T }
fn grow<T>(n: int) -> int { if n == 0 { 0 } else { grow::<Box<T>>(n - 1) } }
fn main() { print(grow::<int>(2)); }
// error: polymorphic recursion: `grow` uses itself with `T` = `Box<T>`
