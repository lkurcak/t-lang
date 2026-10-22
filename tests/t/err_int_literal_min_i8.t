// `-128` is a literal, and fits in an `i8`, but `128` doesn't.
fn main() {
    let _x: i8 = -128;
    let _y: i8 = 128;
}
// error: `128` doesn't fit in an `i8`, whose values are -128 to 127
