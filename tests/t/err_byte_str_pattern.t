fn f(b: &[u8]) -> u8 {
    match b {
        b"x" => 1,
        _ => 0,
    }
}
fn main() {}
// error: a byte string literal can't be a pattern yet; compare with `==`
