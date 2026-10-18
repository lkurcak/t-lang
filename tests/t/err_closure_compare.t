// Closures can't be compared, as functions can't.
fn main() {
    let n = 1;
    let a = || n;
    print_bool(a == a);
}
// error: cannot compare closures with `==`
