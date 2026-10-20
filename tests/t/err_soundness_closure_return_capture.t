// What a closure returns of what it captured lives as long as that does,
// not as long as its argument: `r` can't outlive `n`.
fn main() {
    let a = 1;
    let r;
    {
        let n = 2;
        let f = |_x: &int| -> &int { &n };
        r = f(&a);
    }
    print(*r);
}
// error: `n` doesn't live long enough
