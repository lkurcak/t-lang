// An exported function's effects are its signature's, across packages too.
fn total(n: i64) -> i64 with {} {
    risky::half(n) // error: `risky::half` may panic, but `total` doesn't say `with panic`
}
fn main() {
    let _h = total(4);
}
