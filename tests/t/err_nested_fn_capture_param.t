// Not even the parameters of the function it's in. A recursive one can't be
// a closure, since a closure's name isn't in scope in its own body, so
// there's no fix.
fn count_down(from: int) -> int {
    fn go(n: int) -> int {
        if n == from { 0 } else { go(n + 1) }
    }
    go(0)
}

fn main() {
    print(count_down(3));
}
// error: can't capture `from` in a nested `fn`; use a closure
