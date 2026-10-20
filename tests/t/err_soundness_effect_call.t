// Calling something that may panic may panic.
fn half(n: int, two: int) -> int with panic { n / two }

fn quarter(n: int) -> int { half(half(n, 2), 2) }

fn main() with panic {
    print(quarter(8));
}
// error: `half` may panic, but `quarter` doesn't say `with panic`
