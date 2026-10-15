fn main() {
    let mut r = &0;
    {
        let x = 1;
        r = &x;
    }
    print(*r);
}
// error: `x` doesn't live long enough: it goes out of scope here while still borrowed
