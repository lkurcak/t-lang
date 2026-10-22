#[derive(Discard)]
enum Opt { None, Some(i64) }
fn f(o: Opt) -> i64 {
    match o {
        Opt::Some(x) => x,
        _ => 0,
        Opt::None => 1,
    }
}
fn main() {
    print(f(Opt::Some(7)));
    print(f(Opt::None));
}
// warning: unreachable pattern
// expect: 7
// expect: 0
