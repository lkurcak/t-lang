// A `loop` at the start of a statement is the whole statement, as an `if`
// is, so it must be a `()`.
fn main() {
    loop { break 5; }
    print(1);
}
// error: expected `()`, found `{integer}`: this `loop` is a statement,
