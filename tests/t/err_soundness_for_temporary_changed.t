// The iterator borrows the head's temporary for the whole loop: the body
// can't change a variable the head lent out either.
fn main() {
    let mut a = [1u8, 2, 3];
    for x in a.as_slice().iter() {
        a[0] = *x;
    }
}
// error: cannot borrow `a` as mutable while `a` is borrowed
