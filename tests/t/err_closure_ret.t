fn main() {
    let f = |x: int| -> bool { x + 1 };
    print_bool(f(1));
}
// error: expected `bool`, found `int`
