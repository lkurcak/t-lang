fn main() {
    let mut x = 1;
    let a = &mut x;
    print(x);
    *a += 1;
}
// error: cannot use `x` while `x` is mutably borrowed
