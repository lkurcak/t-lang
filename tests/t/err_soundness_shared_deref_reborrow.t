// A `&mut` can't be made of what's behind a `&mut` behind a `&`.
fn main() {
    let mut x: usize = 1;
    let r = &mut x;
    let a = &r;
    let m: &mut usize = &mut **a;
    *m = 4;
}
// error: cannot borrow `**a` as mutable: it's behind a `&` reference, `a`
