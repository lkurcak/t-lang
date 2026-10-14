// An open end can be narrowed too, keeping the `..=` style.
fn f(n: int) -> int {
    match n {
        0.. => 1,
        ..=10 => 2,
    }
}
fn main() {
    print(f(0));
    print(f(-1));
}
// warning: this range can be narrowed to `..=-1`
// expect: 1
// expect: 2
