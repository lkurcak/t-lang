enum Opt { None, Some(int) }
fn f(o: Opt) -> int {
    match o {
        Opt::Some(x) => x,
        _ => 0,
        Opt::None => 1,
    }
}
fn main() {}
// error: unreachable pattern
