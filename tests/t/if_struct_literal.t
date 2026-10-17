// In the head of `if` and `match`, `x {` starts the body, not a struct literal.
// Parenthesize to use a struct literal there.

#[derive(Eq)]
struct P { x: int }

fn main() {
    let x = true;
    if x { print(1) }                          // expect: 1
    let p = P { x: 3 };
    if p == (P { x: 3 }) { print(2) }          // expect: 2
    match (P { x: 4 }) { P { x } => print(x) } // expect: 4
}
