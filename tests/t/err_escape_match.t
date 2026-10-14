// Through `if` and `match`, any branch can be the one that escapes.
fn pick(n: int, b: bool) -> fn(int) -> int {
    match b {
        true => |x| x,
        false => if n > 0 { |x| x + n } else { |x| x - 1 },
    }
}
fn main() { print(pick(1, false)(1)); }
// error: cannot return a closure that captures `n`
