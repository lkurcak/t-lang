// Unlike C, there's no "truthiness": the condition must be a bool.
fn main() {
    if 1 { print(1) }
}
// error: expected `bool`, found `int`
