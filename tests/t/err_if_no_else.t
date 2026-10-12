// Without an `else` there'd be no value when the condition is false.
fn main() -> int {
    if true { 1 }
}
// error: `if` without `else` must have type `()`, found `int`
