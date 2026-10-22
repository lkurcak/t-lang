fn main() {
    let f = |x: i64| -> bool { x + 1 };
    print_bool(f(1));
}
// error: expected `bool`, found `i64`
