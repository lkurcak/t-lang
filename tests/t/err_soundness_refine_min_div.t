// `d != 0` isn't enough: the least `i64` divided by -1 overflows.
fn div(a: i64, d: { d: i64 | d != 0 }) -> i64 { a / d }

fn main() with panic {
    print(div(-9223372036854775807 - 1, -1));
}
// error: `/` may panic, but `div` doesn't say `with panic`
