// A closure holding a `&mut` isn't `Copy`: a copy would be a second `&mut`
// to the same `n`.
fn main() with panic {
    let mut n: i64 = 0;
    let mut a = || n = n + 1;
    let mut b = a;
    a();
    b();
    print(n);
}
// error: use of moved value `a`
