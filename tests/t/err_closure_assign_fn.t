// A variable holding a function can't be given a closure that captures.
fn main() {
    let mut f = |x: i64| x;
    let n = 1;
    f = |x| x + n;
    print(f(1));
}
// error: expected `fn(i64) -> i64`, found a closure that captures `n`
