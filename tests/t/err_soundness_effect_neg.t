// `-n` overflows at the smallest `i64`, which has no positive partner.
fn neg(n: i64) -> i64 { -n }

fn main() with panic {
    print(neg(-9223372036854775807 - 1));
}
// error: `-` may panic, but `neg` doesn't say `with panic`
