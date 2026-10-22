// `%` by zero panics, as `/` does.
fn rem(a: i64, b: i64) -> i64 { a % b }

fn main() with panic {
    print(rem(1, 0));
}
// error: `%` may panic, but `rem` doesn't say `with panic`
