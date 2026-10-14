enum Opt { None, Some(int) }
fn f(o: Opt) -> int {
    match o {
        Opt::Some(0..=5) => 1,
        Opt::Some(5..10) => 2,
        _ => 3,
    }
}
fn main() {}
// error: this range can be narrowed to `6..10`
