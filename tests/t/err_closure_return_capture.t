// A closure's copy of what it captures goes away when it returns, so it
// can't return a reference to it.
fn main() {
    let n = 1;
    let f = |_r: &int| -> &int { &n };
    let m = 2;
    print(*f(&m));
}
// error: `n` doesn't live long enough
