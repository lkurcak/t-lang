// A string literal is as long as its text: its bytes, not its characters.
fn main() {
    let n: { n: usize | n == 2 } = "hé".len();
    let _ = n;
}
// error: can't prove `n == 2` for `n`
