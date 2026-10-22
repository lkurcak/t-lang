// `+=` is `+`, and must fit too.
fn f(x: { v: i64 | v >= 0 }) -> i64 {
    let mut y = x;
    y += 1;
    y
}

fn main() with panic {
    print(f(9223372036854775807));
}
// error: `+=` may panic, but `f` doesn't say `with panic`
