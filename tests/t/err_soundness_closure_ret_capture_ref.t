// A closure's copy of what it captures goes away when it returns.
fn main() {
    let n = 5;
    let f = || &n;
    print(*f());
}
// error: `n` doesn't live long enough: it goes away when the closure returns
