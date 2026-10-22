// `&` is on integers; on `bool`, `&&` says it.
fn main() {
    let _b = true & false;
}
// error: `&` works on integers, not on `bool`
