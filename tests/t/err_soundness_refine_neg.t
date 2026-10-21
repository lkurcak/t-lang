// `-x` overflows at the least `int`, so it needs `x` to be greater.
fn neg(x: { v: int | v <= 0 }) -> int { -x }

fn main() with panic {
    print(neg(-9223372036854775807 - 1));
}
// error: `-` may panic, but `neg` doesn't say `with panic`
