// Without an `else` there'd be no value when the condition is false.
fn main() {
    let _x: i64 = if true { 1 };
}
// error: `if` without `else` must have type `()`, found `{integer}`
