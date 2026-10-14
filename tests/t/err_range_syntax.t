fn f(n: int) -> int {
    match n { ..=x => 1, _ => 2 }
}
fn main() {}
// error: expected an integer, found `x`
