// Slices can't be compared with `==` yet.
fn main() {
    let a: &[i64] = core::slice::empty();
    let b: &[i64] = core::slice::empty();
    print_bool(a == b);
}
// error: cannot compare `&[i64]` with `==`: that needs `[i64]: Eq`
