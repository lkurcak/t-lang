fn f(b: bool) -> int {
    match b { 0..1 => 1, _ => 2 }
}
fn main() {}
// error: expected `bool`, found `int`
