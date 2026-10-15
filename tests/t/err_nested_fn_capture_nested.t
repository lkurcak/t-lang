// Nor the locals of a function further out.
fn main() {
    let n = 1;
    fn outer() -> int {
        fn inner() -> int { n }
        inner()
    }
    print(outer());
}
// error: can't capture `n` in a nested `fn`; use a closure
