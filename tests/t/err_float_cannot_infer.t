// As an integer literal, a float literal has no default type.
fn main() {
    let x = 1.5;
    // error: cannot fully infer the type of this float, only that it's `{float}`; say which floating-point type it is, as `2.5f64` does
}
