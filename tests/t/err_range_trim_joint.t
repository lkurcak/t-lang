// Two earlier arms cover the start between them.
fn f(n: int) -> int {
    match n {
        0..10 => 1,
        10..20 => 2,
        5..30 => 3,
        _ => 4,
    }
}
fn main() {}
// error: this range can be narrowed to `20..30`
