// `d != 0` isn't enough: the least `int` divided by -1 overflows.
fn div(a: int, d: { d: int | d != 0 }) -> int { a / d }

fn main() with panic {
    print(div(-9223372036854775807 - 1, -1));
}
// error: `/` may panic, but `div` doesn't say `with panic`
