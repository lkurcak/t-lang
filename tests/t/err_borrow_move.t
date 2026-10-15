fn main() {
    let b = Box::new(1);
    let r = &b;
    let c = b;
    print(**r);
    print(*c);
}
// error: cannot move out of `b` while `b` is borrowed
