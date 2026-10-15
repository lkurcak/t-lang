// Two functions of the same name in one block, as at the top level.
fn main() {
    fn f() {}
    fn f() {}
    f();
}
// error: `f` is already defined
