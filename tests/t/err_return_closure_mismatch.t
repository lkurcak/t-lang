// The closure's result type is worked out from its `return` and its body,
// which must agree.
fn main() {
    let f = |x: int| {
        if x > 0 { return true; }
        x
    };
    print_bool(f(1));
}
// error: this closure returns `bool`, but its body is `int`
