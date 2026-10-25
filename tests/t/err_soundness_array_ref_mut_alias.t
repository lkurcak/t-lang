// Two `&mut`s to one place, through an array: the second borrow conflicts
// with the first while the array holding it is used.
fn main() {
    let mut x: i64 = 1;
    let a = [&mut x];
    let b = &mut x;
    *b = 2;
    *a[0] = 3;
}
// error: cannot borrow `x` as mutable more than once at a time
