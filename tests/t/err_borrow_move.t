fn main() with alloc {
    let b = Box::new(1i64);
    let r = &b;
    let c = b;
    print(**r);
    print(*c);
}
// error: cannot move out of `b` while `b` is borrowed
