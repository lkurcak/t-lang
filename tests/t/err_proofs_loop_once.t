// A proof that fails in a loop's body is reported once, however many
// times the checker walks the body.
fn need(s: { s: u8 | s < 64 }) -> u8 { s }

fn sum(x: u8, n: u8) -> u8 {
    let mut s: u8 = 0;
    for _ in 0..n {
        s = need(x);
    }
    s
}

fn main() {
    let _ = sum(1, 2);
}
// error: can't prove `s < 64` for argument `s` of `need`
