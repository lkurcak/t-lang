fn main() {
    let mut x = 1;
    let a = &mut x;
    let b = &mut x;
    *a += 1;
    *b += 1;
}
// error: cannot borrow `x` as mutable more than once at a time
