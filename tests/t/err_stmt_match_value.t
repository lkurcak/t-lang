fn main() -> i64 {
    match true { true => 1, false => 2 }
    + 1
}
// error: expected an expression, found `+`
