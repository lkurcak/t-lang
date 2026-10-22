// Nothing says which type `5` is, and there's no default.
fn main() {
    let x = 5;
    print_bool(x == 5);
}
// error: cannot infer the type of this integer
