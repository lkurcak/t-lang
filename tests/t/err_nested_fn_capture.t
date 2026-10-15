// A nested `fn` can't use the locals around it: it means what it would at
// the top level. The fix makes it a closure, which can.
fn main() {
    let n = 2;
    /// Adds `n`.
    fn add_n(x: int) -> int { x + n }
    print(add_n(1));
}
// error: can't capture `n` in a nested `fn`; use a closure
