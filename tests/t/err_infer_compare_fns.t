// `==` on a type not known yet is checked once it is.
fn main() {
    let eq = |a, b| a == b;
    print_bool(eq(main, main));
}
// error: cannot compare functions with `==`
