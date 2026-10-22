// None of these is an unused variable.
#[derive(Discard)]
struct P { x: i64, y: i64 }
fn ignore(_n: i64) -> i64 { 0 }
fn main() {
    let _later: i64 = 1;
    let P { x, y: _ } = P { x: 2, y: 3 };
    // Used only in the next `let`, which shadows it.
    let a: i64 = 4;
    let a = a + 1;
    print(a);
    print(x);
    print(ignore(6));
}
// expect: 5
// expect: 2
// expect: 0
