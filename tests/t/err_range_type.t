fn f(b: bool) -> i64 {
    match b { 0..1 => 1, _ => 2 }
}
fn main() {}
// error: expected `bool`, found `{integer}`
