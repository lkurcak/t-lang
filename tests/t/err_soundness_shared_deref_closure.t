// A closure that captures a `&` to a `&mut` can't write through it.
fn main() {
    let mut x: usize = 1;
    let r = &mut x;
    let a = &r;
    let mut f = || {
        **a = 7;
    };
    f();
}
// error: it's behind a `&` reference
