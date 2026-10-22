// Calling something that may panic may panic.
fn half(n: i64, two: i64) -> i64 with panic { n / two }

fn quarter(n: i64) -> i64 { half(half(n, 2), 2) }

fn main() with panic {
    print(quarter(8));
}
// error: `half` may panic, but `quarter` doesn't say `with panic`
