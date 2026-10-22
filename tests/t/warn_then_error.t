// Warnings found before an error are still reported.
fn f(n: i64) -> i64 {
    match n {
        0..10 | 10..20 => 1,
        _ => 2,
    }
}
fn main() {
    let x: i64 = true;
}
// warning: can be merged into `0..20`
// error: expected `i64`, found `bool`
