// Nothing is past `10FFFF`.
fn main() {
    print_str("\u{110000}");
}
// error: `\u{110000}` isn't a Unicode scalar value
