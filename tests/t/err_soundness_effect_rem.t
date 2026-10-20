// `%` by zero panics, as `/` does.
fn rem(a: int, b: int) -> int { a % b }

fn main() with panic {
    print(rem(1, 0));
}
// error: `%` may panic, but `rem` doesn't say `with panic`
