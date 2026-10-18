// The `1` is covered by the arm above. The arm as a whole is still
// reachable (through `2` and `None`), so this is only visible per alternative.
#[derive(Discard)]
enum Opt { None, Some(int) }
fn f(o: Opt) -> int {
    match o {
        Opt::Some(1) => 1,
        Opt::Some(1 | 2) | Opt::None => 2,
        _ => 0,
    }
}
fn main() {
    print(f(Opt::Some(1)));
    print(f(Opt::Some(2)));
    print(f(Opt::None));
    print(f(Opt::Some(3)));
}
// warning: unreachable alternative
// expect: 1
// expect: 2
// expect: 2
// expect: 0
