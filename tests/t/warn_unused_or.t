// A variable bound by an or-pattern is one variable, with one warning, and
// the fix renames it in every alternative.
#[derive(Discard)]
enum Pair { L(i64, i64), R(i64, i64) }
fn f(p: Pair) -> i64 {
    match p {
        Pair::L(a, b) | Pair::R(b, a) => a,
    }
}
fn main() {
    print(f(Pair::R(1, 2)));
    print(f(Pair::L(3, 4)));
}
// warning: unused variable `b`
// expect: 2
// expect: 3
