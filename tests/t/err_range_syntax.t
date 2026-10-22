fn f(n: i64) -> i64 {
    match n { ..=x => 1, _ => 2 }
}
fn main() {}
// error: expected an integer, found `x`
