// Warnings found before an error are still reported.
fn f(n: int) -> int {
    match n {
        0..10 | 10..20 => 1,
        _ => 2,
    }
}
fn main() {
    let x: int = true;
}
// warning: can be merged into `0..20`
// error: expected `int`, found `bool`
