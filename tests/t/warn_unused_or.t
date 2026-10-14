// A variable bound by an or-pattern is one variable, with one warning, and
// the fix renames it in every alternative.
enum Pair { L(int, int), R(int, int) }
fn f(p: Pair) -> int {
    match p {
        Pair::L(a, b) | Pair::R(b, a) => a,
    }
}
fn main() {
    print(f(Pair::R(1, 2)));
}
// warning: unused variable `b`
// expect: 2
