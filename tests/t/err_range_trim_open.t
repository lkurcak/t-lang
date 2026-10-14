// An open end can be narrowed too, keeping the `..=` style.
fn f(n: int) -> int {
    match n {
        0.. => 1,
        ..=10 => 2,
    }
}
fn main() {}
// error: this range can be narrowed to `..=-1`
