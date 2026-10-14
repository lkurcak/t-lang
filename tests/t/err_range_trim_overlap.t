// The first arm already matches 10..20, so the second only ever gets 20..30.
fn f(n: int) -> int {
    match n {
        0..20 => 1,
        10..30 => 2,
        _ => 3,
    }
}
fn main() {}
// error: this range can be narrowed to `20..30`
