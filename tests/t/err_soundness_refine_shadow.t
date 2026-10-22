// A new `d` is a new variable: what was known of the old one isn't of it.
fn main() {
    let d: i64 = 1;
    let d = d - 1;
    print(10 / d);
}
// error: `/` may panic, but `main` doesn't say `with panic`
