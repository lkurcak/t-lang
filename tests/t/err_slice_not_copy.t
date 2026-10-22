// A `&mut [T]` isn't `Copy`, as a `&mut T` isn't: using one reborrows it,
// so the original can't be used while what it was given to lives.
fn main() {
    let s: &mut [i64] = core::slice::empty_mut();
    let t = s;
    s.len();
    t.len();
}
// error: cannot borrow `*s` while `*s` is mutably borrowed
