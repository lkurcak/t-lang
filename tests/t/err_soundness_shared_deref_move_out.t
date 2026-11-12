// Moving a `&mut` out of a `&` reborrows it, which a `&` can't.
fn main() {
    let mut x: usize = 1;
    let r = &mut x;
    let a = &r;
    let m: &mut usize = *a;
    *m = 3;
}
// error: it's behind a `&` reference, `a`
