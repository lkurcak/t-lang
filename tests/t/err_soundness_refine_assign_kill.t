// After `d = 0`, what was known of `d` before is about a value it no
// longer has.
fn main() {
    let mut d = 1;
    d = 0;
    print(10 / d);
}
// error: `/` may panic, but `main` doesn't say `with panic`
