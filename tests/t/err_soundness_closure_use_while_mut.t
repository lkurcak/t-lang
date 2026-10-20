// While a closure that changes `n` is still to be called, `n` is borrowed
// mutably by it, so it can't be read.
fn main() with panic {
    let mut n = 0;
    let mut inc = || n = n + 1;
    print(n);
    inc();
}
// error: cannot use `n` while `n` is mutably borrowed
