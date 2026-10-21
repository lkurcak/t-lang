// Every `int` is at most the greatest, which isn't enough for `+ 1`.
fn inc(x: { v: int | v <= 9223372036854775807 }) -> int { x + 1 }

fn main() with panic {
    print(inc(9223372036854775807));
}
// error: `+` may panic, but `inc` doesn't say `with panic`
