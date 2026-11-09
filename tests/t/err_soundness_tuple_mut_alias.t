// A `&mut` to one field and a `&` to the whole can't both live.
fn main() {
    let mut t: (u8, u8) = (1, 2);
    let a = &mut t.0;
    let b = &t;
    *a = 5;
    let _x = b.0;
}
// error: cannot borrow `t` while `t.0` is mutably borrowed
