// Nor the locals of a function further out.
fn main() {
    let n = 1;
    fn outer() -> i64 {
        fn inner() -> i64 { n }
        inner()
    }
    print(outer());
}
// error: can't capture `n` in a nested `fn`; use a closure
