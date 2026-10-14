// The other side of empty_enum.t: an arm for a variant that can't hold a
// value can never run.
enum Never {}
enum Maybe { Real(int), Impossible(Never) }
fn get(m: Maybe) -> int {
    match m {
        Maybe::Real(x) => x,
        Maybe::Impossible(n) => match n {},
    }
}
fn main() {
    print(get(Maybe::Real(4)));
}
// warning: variant `Maybe::Impossible` is never constructed
// warning: unreachable pattern
// expect: 4
