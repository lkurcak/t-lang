// A remainder has the sign of what's divided, so this one can be negative.
fn rem(a: i64, b: { b: i64 | b > 0 }) -> { r: i64 | r >= 0 } {
    a % b
}

fn main() {
    let _ = rem(-7, 3);
}
// error: can't prove `r >= 0` for the result of `rem`
