fn main() {
    let mut x: i64 = 1;
    let r = &x;
    x = 2;
    print(*r);
}
// error: cannot assign to `x` while `x` is borrowed
