// `-x` overflows at the least `i64`, so it needs `x` to be greater.
fn neg(x: { v: i64 | v <= 0 }) -> i64 { -x }

fn main() with panic {
    print(neg(-9223372036854775807 - 1));
}
// error: `-` may panic, but `neg` doesn't say `with panic`
