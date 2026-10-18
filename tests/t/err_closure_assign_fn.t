// A variable holding a function can't be given a closure that captures.
fn main() {
    let mut f = |x: int| x;
    let n = 1;
    f = |x| x + n;
    print(f(1));
}
// error: expected `fn(int) -> int`, found a closure that captures `n`
