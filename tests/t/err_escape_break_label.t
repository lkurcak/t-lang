// The `break 'outer` is inside the inner loop, but it gives the outer one
// its value, and the outer one is returned: the closure would leave the
// frame `n` is in.
fn make(n: int) -> fn(int) -> int {
    'outer: loop {
        loop {
            break 'outer |x| x + n;
        }
    }
}
fn main() {
    print(make(1)(2));
}
// error: cannot return a closure that captures `n`
