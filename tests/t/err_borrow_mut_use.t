fn main() with panic {
    let mut x: i64 = 1;
    let a = &mut x;
    let _v = x;
    *a += 1;
}
// error: cannot use `x` while `x` is mutably borrowed
