// Slices can't be compared with `==` yet.
fn main() {
    let a: &[int] = core::slice::empty();
    let b: &[int] = core::slice::empty();
    print_bool(a == b);
}
// error: cannot compare `&[int]` with `==`: that needs `[int]: Eq`
