#[derive(Discard)]
enum Opt { None, Some(int) }
fn f(o: Opt) -> int {
    match o {
        Opt::Some(0..=5) => 1,
        Opt::Some(5..10) => 2,
        _ => 3,
    }
}
fn main() {
    print(f(Opt::Some(5)));
    print(f(Opt::Some(6)));
    print(f(Opt::None));
}
// warning: this range can be narrowed to `6..10`
// expect: 1
// expect: 2
// expect: 3
