// None of these is an unused variable.
#[derive(Discard)]
struct P { x: int, y: int }
fn ignore(_n: int) -> int { 0 }
fn main() with panic {
    let _later = 1;
    let P { x, y: _ } = P { x: 2, y: 3 };
    // Used only in the next `let`, which shadows it.
    let a = 4;
    let a = a + 1;
    print(a);
    print(x);
    print(ignore(6));
}
// expect: 5
// expect: 2
// expect: 0
