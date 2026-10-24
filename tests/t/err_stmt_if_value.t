// The `if` is its own statement, so `- x` is not a subtraction. In C this
// would compute 10 - 5; here (and in Rust) it is a type error.
fn f() -> i64 {
    let x = 5;
    if x > 0 { 10 } else { 20 }
    - x
}

fn main() { let _x = f(); }
// error: expected `()`, found `{integer}`: this `if` is a statement,
