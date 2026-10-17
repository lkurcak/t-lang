// An early `return` is a way out of the frame as much as the function's
// result is.
fn make(n: int) -> fn(int) -> int {
    if n > 0 { return |x| x + n; }
    |x| x
}
fn main() {
    print(make(1)(2));
}
// error: cannot return a closure that captures `n`
