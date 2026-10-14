// The `1` is covered by the arm above. The arm as a whole is still
// reachable (through `2` and `None`), so this is only visible per alternative.
enum Opt { None, Some(int) }
fn f(o: Opt) -> int {
    match o {
        Opt::Some(1) => 1,
        Opt::Some(1 | 2) | Opt::None => 2,
        _ => 0,
    }
}
fn main() {}
// error: unreachable alternative
