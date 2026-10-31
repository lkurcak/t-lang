fn f() -> i64 {
    let n: i64 else { return 0 };
    n
}

fn main() {
    let _ = f();
}
// error: syntax error
