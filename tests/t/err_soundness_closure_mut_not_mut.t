// Calling a closure that changes what it captured borrows it mutably.
fn main() with panic {
    let mut n = 0;
    let inc = || n = n + 1;
    inc();
    print(n);
}
// error: cannot borrow `inc` as mutable, as `inc` isn't declared `mut`
