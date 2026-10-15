fn main() {
    let mut x = 1;
    let a = &x;
    let b = &mut x;
    *b += 1;
    print(*a);
}
// error: cannot borrow `x` as mutable while `x` is borrowed
