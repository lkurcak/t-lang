// What's divided bounds the remainder only by `<=`: `3 % 5` is `3`.
fn rem(a: u64, n: { n: u64 | n > 0 }) -> { r: u64 | r < a } {
    a % n
}

fn main() {
    let _ = rem(3, 5);
}
// error: can't prove `r < a` for the result of `rem`
