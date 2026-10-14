// Not a top-level alternative, so checking those alone would miss it.
enum Opt { None, Some(int) }
fn f(o: Opt) -> int {
    match o {
        Opt::Some(1 | 1) => 1,
        _ => 0,
    }
}
fn main() {}
// error: unreachable alternative
