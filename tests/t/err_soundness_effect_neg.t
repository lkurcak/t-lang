// `-n` overflows at the smallest `int`, which has no positive partner.
fn neg(n: int) -> int { -n }

fn main() with panic {
    print(neg(-9223372036854775807 - 1));
}
// error: `-` may panic, but `neg` doesn't say `with panic`
