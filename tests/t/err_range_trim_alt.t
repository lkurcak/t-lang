// Earlier alternatives count, as for unreachable alternatives.
fn f(n: int) -> int {
    match n {
        0..10 | 5..20 => 1,
        _ => 2,
    }
}
fn main() {}
// error: this range can be narrowed to `10..20`
