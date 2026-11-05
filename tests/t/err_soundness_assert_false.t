// `assert` ensures what it was given, nothing more.
fn third(s: &[u8]) -> u8 {
    assert(s.len() >= 2);
    s[2]
}
fn main() {
    let _ = third([1u8, 2, 3].as_slice());
}
// error: can't prove `i < self.len()` for argument `i` of `index`
