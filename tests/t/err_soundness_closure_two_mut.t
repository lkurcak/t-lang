// Two closures that both change `n` would each hold a `&mut n`.
fn main() with panic {
    let mut n: i64 = 0;
    let mut a = || n = n + 1;
    let mut b = || n = n + 2;
    a();
    b();
    print(n);
}
// error: cannot borrow `n` as mutable more than once at a time
