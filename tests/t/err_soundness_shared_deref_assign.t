// A `&mut` behind a `&` is shared: nothing may be written through it.
fn main() {
    let mut x: usize = 1;
    let r = &mut x;
    let a = &r;
    let b = &r;
    **a = 5;
    let _y = **b;
}
// error: cannot assign to `**a`: it's behind a `&` reference, `a`
