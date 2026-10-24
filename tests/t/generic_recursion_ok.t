// Recursion that changes a type argument is fine, as long as it doesn't
// keep growing: `count::<bool>` needs `count::<i64>`, which needs itself.
fn count<T: Discard>(_x: T, n: i64) -> i64 with panic {
    if n == 0 { 0 } else { 1 + count(n, n - 1) }
}
// So is swapping type parameters round.
fn flip<A: Discard, B: Discard>(a: A, b: B, n: i64) -> i64 with panic {
    if n == 0 { 0 } else { 1 + flip(b, a, n - 1) }
}
fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&count(true, 5))?;      // expect: 5
    out.line(&flip(1i64, false, 4))?;   // expect: 4
    Result::Ok(())
}
